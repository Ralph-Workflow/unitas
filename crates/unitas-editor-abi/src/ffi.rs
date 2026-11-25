use crate::descriptor::{Scope, ToolDescriptor};
use crate::dispatch::{error_json, AbiError, DispatchTable};
use serde_json::json;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock, RwLock};

static TABLE: RwLock<Option<DispatchTable>> = RwLock::new(None);
static STRINGS: OnceLock<Mutex<HashMap<usize, Box<[u8]>>>> = OnceLock::new();

const PANIC_MESSAGE: &str = "internal panic in unitas-editor-abi";

pub fn builtin_dispatch_table() -> Result<DispatchTable, AbiError> {
    let descriptor = ToolDescriptor {
        name: "engine.abi_status".to_string(),
        description: "Inspect unitas-editor-abi native tool registration status".to_string(),
        input_schema: json!({"type": "object", "properties": {}}),
        required_scope: Scope::ReadProject,
        origin: "unitas://native/editor-abi".to_string(),
    };
    let mut builder = DispatchTable::builder();
    for scope in Scope::ALL {
        builder = builder.grant(scope);
    }
    Ok(builder
        .tool(
            descriptor,
            Box::new(|args| {
                Ok(json!({
                    "abi": "unitas-editor-abi",
                    "status": "ready",
                    "echo": args,
                }))
            }),
        )?
        .build())
}

pub fn install_dispatch_table(table: DispatchTable) -> Result<(), AbiError> {
    let mut guard = TABLE.write().map_err(|_| AbiError::Poisoned)?;
    *guard = Some(table);
    Ok(())
}

pub fn ensure_default_dispatch_table() -> Result<(), AbiError> {
    let mut guard = TABLE.write().map_err(|_| AbiError::Poisoned)?;
    if guard.is_none() {
        *guard = Some(builtin_dispatch_table()?);
    }
    Ok(())
}

fn string_registry() -> &'static Mutex<HashMap<usize, Box<[u8]>>> {
    STRINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn allocate_c_string(text: &str) -> usize {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    for &byte in text.as_bytes() {
        if byte == 0 {
            return 0;
        }
        bytes.push(byte);
    }
    bytes.push(0);
    let boxed = bytes.into_boxed_slice();
    let address = boxed.as_ptr() as usize;
    let Ok(mut map) = string_registry().lock() else {
        return 0;
    };
    map.insert(address, boxed);
    address
}

fn read_c_string(address: usize) -> Option<Result<String, ()>> {
    if address == 0 {
        return None;
    }
    let map = string_registry().lock().ok()?;
    let bytes = map.get(&address)?;
    let payload = bytes.split_last().map_or(&[][..], |(_, rest)| rest);
    Some(
        std::str::from_utf8(payload)
            .map(str::to_owned)
            .map_err(|_| ()),
    )
}

fn panic_error() -> usize {
    allocate_c_string(&error_json("internal", None, PANIC_MESSAGE).to_string())
}

fn with_table(f: impl FnOnce(Option<&DispatchTable>) -> String) -> String {
    match TABLE.read() {
        Ok(guard) => f(guard.as_ref()),
        Err(_) => error_json("internal", None, "dispatch table lock poisoned").to_string(),
    }
}

fn describe_json() -> String {
    with_table(|table| {
        table.map_or_else(
            || "{\"tools\":[]}".to_string(),
            DispatchTable::describe_json,
        )
    })
}

fn invoke_json(name_addr: usize, args_addr: usize) -> String {
    if name_addr == 0 {
        return error_json("invalid-args", None, "tool name pointer is null").to_string();
    }
    let Some(Ok(name)) = read_c_string(name_addr) else {
        return error_json("invalid-args", None, "tool name is not valid UTF-8").to_string();
    };
    let args = if args_addr == 0 {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        let Some(Ok(raw)) = read_c_string(args_addr) else {
            return error_json("invalid-args", Some(&name), "args are not valid UTF-8 JSON")
                .to_string();
        };
        let Ok(value) = serde_json::from_str(&raw) else {
            return error_json("invalid-args", Some(&name), "args are not valid UTF-8 JSON")
                .to_string();
        };
        value
    };
    with_table(|table| match table {
        Some(table) => table.invoke(&name, &args).to_string(),
        None => {
            error_json("unknown-tool", Some(&name), &format!("unknown tool {name}")).to_string()
        }
    })
}

pub extern "C" fn unitas_editor_install_builtin_tools() -> i32 {
    catch_unwind(|| match ensure_default_dispatch_table() {
        Ok(()) => 0,
        Err(_) => -1,
    })
    .unwrap_or(-1)
}

pub extern "C" fn unitas_editor_tools_describe() -> usize {
    match catch_unwind(|| allocate_c_string(&describe_json())) {
        Ok(ptr) => ptr,
        Err(_) => panic_error(),
    }
}

pub extern "C" fn unitas_editor_tool_invoke(name: usize, args_json: usize) -> usize {
    match catch_unwind(AssertUnwindSafe(|| {
        allocate_c_string(&invoke_json(name, args_json))
    })) {
        Ok(ptr) => ptr,
        Err(_) => panic_error(),
    }
}

pub extern "C" fn unitas_editor_string_free(ptr: usize) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if ptr != 0 {
            if let Ok(mut map) = string_registry().lock() {
                map.remove(&ptr);
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{Scope, ToolDescriptor};
    use serde_json::{json, Value};

    static FFI_LOCK: Mutex<()> = Mutex::new(());

    fn lock_ffi() -> std::sync::MutexGuard<'static, ()> {
        FFI_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn read_and_free(ptr: usize) -> Result<Value, Box<dyn std::error::Error>> {
        if ptr == 0 {
            return Err("null string pointer".into());
        }
        let text = match read_c_string(ptr) {
            Some(Ok(text)) => text,
            _ => return Err("invalid UTF-8 string pointer".into()),
        };
        unitas_editor_string_free(ptr);
        Ok(serde_json::from_str(&text)?)
    }

    fn install_test_table() -> Result<(), AbiError> {
        let descriptor = ToolDescriptor {
            name: "test.echo".to_string(),
            description: "Echo args".to_string(),
            input_schema: json!({"type": "object"}),
            required_scope: Scope::ReadProject,
            origin: "unitas://test".to_string(),
        };
        let table = DispatchTable::builder()
            .grant(Scope::ReadProject)
            .tool(descriptor, Box::new(|args| Ok(args.clone())))?
            .build();
        install_dispatch_table(table)
    }

    #[test]
    fn ffi_describe_invoke_and_error_paths() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = lock_ffi();
        install_test_table()?;

        let described = read_and_free(unitas_editor_tools_describe())?;
        assert_eq!(described["tools"][0]["name"], json!("test.echo"));
        assert_eq!(
            described["tools"][0]["requiredScope"],
            json!("read-project")
        );

        let name = allocate_c_string("test.echo");
        let args = allocate_c_string("{\"x\": 7}");
        let invoked = read_and_free(unitas_editor_tool_invoke(name, args))?;
        assert_eq!(invoked, json!({"x": 7}));

        let unknown = allocate_c_string("nope");
        let result = read_and_free(unitas_editor_tool_invoke(unknown, args))?;
        assert_eq!(result["error"]["kind"], json!("unknown-tool"));
        unitas_editor_string_free(unknown);

        let malformed = allocate_c_string("{not json");
        let result = read_and_free(unitas_editor_tool_invoke(name, malformed))?;
        assert_eq!(result["error"]["kind"], json!("invalid-args"));
        unitas_editor_string_free(malformed);

        let result = read_and_free(unitas_editor_tool_invoke(0, args))?;
        assert_eq!(result["error"]["kind"], json!("invalid-args"));
        unitas_editor_string_free(name);
        unitas_editor_string_free(args);
        Ok(())
    }

    #[test]
    fn ffi_missing_scope_returns_structured_error() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = lock_ffi();
        let descriptor = ToolDescriptor {
            name: "test.gated".to_string(),
            description: "Needs build scope".to_string(),
            input_schema: json!({}),
            required_scope: Scope::Build,
            origin: "unitas://test".to_string(),
        };
        let table = DispatchTable::builder()
            .tool(descriptor, Box::new(|_| Ok(json!({}))))?
            .build();
        install_dispatch_table(table)?;

        let name = allocate_c_string("test.gated");
        let result = read_and_free(unitas_editor_tool_invoke(name, 0))?;
        unitas_editor_string_free(name);
        assert_eq!(result["error"]["kind"], json!("missing-scope"));
        assert_eq!(result["error"]["scope"], json!("build"));
        Ok(())
    }

    #[test]
    fn ffi_panicking_handler_does_not_cross_the_boundary() -> Result<(), Box<dyn std::error::Error>>
    {
        let _guard = lock_ffi();
        let descriptor = ToolDescriptor {
            name: "test.panic".to_string(),
            description: "Panics".to_string(),
            input_schema: json!({}),
            required_scope: Scope::ReadProject,
            origin: "unitas://test".to_string(),
        };
        let table = DispatchTable::builder()
            .grant(Scope::ReadProject)
            .tool(
                descriptor,
                Box::new(|_| -> Result<Value, String> { unreachable!("handler exploded") }),
            )?
            .build();
        install_dispatch_table(table)?;

        let name = allocate_c_string("test.panic");
        let result = read_and_free(unitas_editor_tool_invoke(name, 0))?;
        unitas_editor_string_free(name);
        assert_eq!(result["error"]["kind"], json!("internal"));
        Ok(())
    }

    #[test]
    fn ffi_install_builtin_tools_describes_and_invokes_abi_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _guard = lock_ffi();
        if let Ok(mut guard) = TABLE.write() {
            *guard = None;
        }
        assert_eq!(unitas_editor_install_builtin_tools(), 0);

        let described = read_and_free(unitas_editor_tools_describe())?;
        assert_eq!(described["tools"][0]["name"], json!("engine.abi_status"));
        assert_eq!(
            described["tools"][0]["origin"],
            json!("unitas://native/editor-abi")
        );

        let name = allocate_c_string("engine.abi_status");
        let args = allocate_c_string("{\"probe\":true}");
        let invoked = read_and_free(unitas_editor_tool_invoke(name, args))?;
        unitas_editor_string_free(name);
        unitas_editor_string_free(args);
        assert_eq!(invoked["status"], json!("ready"));
        assert_eq!(invoked["echo"]["probe"], json!(true));
        Ok(())
    }
}
