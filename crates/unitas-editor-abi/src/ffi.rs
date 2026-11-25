use crate::dispatch::{error_json, AbiError, DispatchTable};
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::RwLock;

static TABLE: RwLock<Option<DispatchTable>> = RwLock::new(None);

const PANIC_MESSAGE: &str = "internal panic in unitas-editor-abi";

pub fn install_dispatch_table(table: DispatchTable) -> Result<(), AbiError> {
    let mut guard = TABLE.write().map_err(|_| AbiError::Poisoned)?;
    *guard = Some(table);
    Ok(())
}

fn into_raw(json: String) -> *mut c_char {
    // Serialized JSON never contains interior NUL bytes.
    match CString::new(json) {
        Ok(string) => string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn panic_error() -> *mut c_char {
    into_raw(
        error_json("internal", None, PANIC_MESSAGE.to_string()).to_string(),
    )
}

fn with_table(f: impl FnOnce(Option<&DispatchTable>) -> String) -> String {
    match TABLE.read() {
        Ok(guard) => f(guard.as_ref()),
        Err(_) => error_json("internal", None, "dispatch table lock poisoned".to_string())
            .to_string(),
    }
}

fn invoke_json(name: *const c_char, args_json: *const c_char) -> String {
    if name.is_null() {
        return error_json("invalid-args", None, "tool name pointer is null".to_string())
            .to_string();
    }
    // SAFETY: caller guarantees `name` is a valid NUL-terminated string.
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name) => name,
        Err(_) => {
            return error_json("invalid-args", None, "tool name is not valid UTF-8".to_string())
                .to_string()
        }
    };
    let args = if args_json.is_null() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        // SAFETY: caller guarantees `args_json` is a valid NUL-terminated string.
        let raw = unsafe { CStr::from_ptr(args_json) };
        match raw.to_str().map(serde_json::from_str) {
            Ok(Ok(value)) => value,
            _ => {
                return error_json(
                    "invalid-args",
                    Some(name),
                    "args are not valid UTF-8 JSON".to_string(),
                )
                .to_string()
            }
        }
    };
    with_table(|table| match table {
        Some(table) => table.invoke(name, &args).to_string(),
        None => error_json("unknown-tool", Some(name), format!("unknown tool {name}"))
            .to_string(),
    })
}

#[no_mangle]
pub extern "C" fn unitas_editor_tools_describe() -> *mut c_char {
    match catch_unwind(|| {
        into_raw(with_table(|table| {
            table.map_or_else(|| "{\"tools\":[]}".to_string(), DispatchTable::describe_json)
        }))
    }) {
        Ok(ptr) => ptr,
        Err(_) => panic_error(),
    }
}

#[no_mangle]
pub extern "C" fn unitas_editor_tool_invoke(
    name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(|| into_raw(invoke_json(name, args_json)))) {
        Ok(ptr) => ptr,
        Err(_) => panic_error(),
    }
}

/// # Safety
///
/// `ptr` must be null or a pointer previously returned by
/// `unitas_editor_tools_describe` or `unitas_editor_tool_invoke`.
#[no_mangle]
pub unsafe extern "C" fn unitas_editor_string_free(ptr: *mut c_char) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !ptr.is_null() {
            // SAFETY: contract above; the pointer came from CString::into_raw.
            drop(unsafe { CString::from_raw(ptr) });
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{Scope, ToolDescriptor};
    use serde_json::{json, Value};
    use std::sync::Mutex;

    // The dispatch table is global; serialize the FFI tests that install into it.
    static FFI_LOCK: Mutex<()> = Mutex::new(());

    fn lock_ffi() -> std::sync::MutexGuard<'static, ()> {
        FFI_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn read_and_free(ptr: *mut c_char) -> Value {
        assert!(!ptr.is_null());
        // SAFETY: ptr is a live CString from an export in this module.
        let text = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .expect("utf-8 JSON")
            .to_string();
        unsafe { unitas_editor_string_free(ptr) };
        serde_json::from_str(&text).expect("valid JSON")
    }

    fn cstring(text: &str) -> CString {
        CString::new(text).expect("no interior nul")
    }

    fn install_test_table() {
        let descriptor = ToolDescriptor {
            name: "test.echo".to_string(),
            description: "Echo args".to_string(),
            input_schema: json!({"type": "object"}),
            required_scope: Scope::ReadProject,
            origin: "unitas://test".to_string(),
        };
        let table = DispatchTable::builder()
            .grant(Scope::ReadProject)
            .tool(descriptor, Box::new(|args| Ok(args.clone())))
            .expect("unique tool")
            .build();
        install_dispatch_table(table).expect("install");
    }

    #[test]
    fn ffi_describe_invoke_and_error_paths() {
        let _guard = lock_ffi();
        install_test_table();

        let described = read_and_free(unitas_editor_tools_describe());
        assert_eq!(described["tools"][0]["name"], json!("test.echo"));
        assert_eq!(described["tools"][0]["requiredScope"], json!("read-project"));

        let name = cstring("test.echo");
        let args = cstring("{\"x\": 7}");
        let invoked = read_and_free(unitas_editor_tool_invoke(name.as_ptr(), args.as_ptr()));
        assert_eq!(invoked, json!({"x": 7}));

        let unknown = cstring("nope");
        let result = read_and_free(unitas_editor_tool_invoke(unknown.as_ptr(), args.as_ptr()));
        assert_eq!(result["error"]["kind"], json!("unknown-tool"));

        let malformed = cstring("{not json");
        let result = read_and_free(unitas_editor_tool_invoke(name.as_ptr(), malformed.as_ptr()));
        assert_eq!(result["error"]["kind"], json!("invalid-args"));

        let result = read_and_free(unitas_editor_tool_invoke(std::ptr::null(), args.as_ptr()));
        assert_eq!(result["error"]["kind"], json!("invalid-args"));
    }

    #[test]
    fn ffi_missing_scope_returns_structured_error() {
        let _guard = lock_ffi();
        let descriptor = ToolDescriptor {
            name: "test.gated".to_string(),
            description: "Needs build scope".to_string(),
            input_schema: json!({}),
            required_scope: Scope::Build,
            origin: "unitas://test".to_string(),
        };
        let table = DispatchTable::builder()
            .tool(descriptor, Box::new(|_| Ok(json!({}))))
            .expect("unique tool")
            .build();
        install_dispatch_table(table).expect("install");

        let name = cstring("test.gated");
        let result = read_and_free(unitas_editor_tool_invoke(name.as_ptr(), std::ptr::null()));
        assert_eq!(result["error"]["kind"], json!("missing-scope"));
        assert_eq!(result["error"]["scope"], json!("build"));
    }

    #[test]
    fn ffi_panicking_handler_does_not_cross_the_boundary() {
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
            .tool(descriptor, Box::new(|_| panic!("handler exploded")))
            .expect("unique tool")
            .build();
        install_dispatch_table(table).expect("install");

        let name = cstring("test.panic");
        let result = read_and_free(unitas_editor_tool_invoke(name.as_ptr(), std::ptr::null()));
        assert_eq!(result["error"]["kind"], json!("internal"));
    }
}
