//! Layout agreement across the editor ABI zone.
//!
//! The six exported symbols of this zone take and return pointer-sized
//! addresses and one four-byte status code, and nothing else, so no struct
//! layout crosses the boundary. The guard below pins the scalar widths, pins
//! every exported signature at compile time so that a struct parameter could
//! never be introduced unnoticed, and pins the scope names, because the
//! manifest, the tool descriptor, and the scope enum cross as UTF-8 JSON text
//! and therefore agree by name rather than by memory layout.

use std::mem::{align_of, size_of};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use unitas_editor_abi::{
    Scope, ShellEventCallback, allocate_c_string, free_c_string, read_c_string,
    unitas_editor_allocate_string, unitas_editor_install_builtin_tools, unitas_editor_run_shell,
    unitas_editor_string_buffer, unitas_editor_string_free, unitas_editor_tool_invoke,
    unitas_editor_tools_describe,
};

const _: extern "C" fn(capacity: usize) -> usize = unitas_editor_string_buffer;
const _: extern "C" fn(address: usize, length: usize) -> usize = unitas_editor_allocate_string;
const _: extern "C" fn() -> i32 = unitas_editor_install_builtin_tools;
const _: extern "C" fn() -> usize = unitas_editor_tools_describe;
const _: extern "C" fn(name: usize, args_json: usize) -> usize = unitas_editor_tool_invoke;
const _: extern "C" fn(ptr: usize) = unitas_editor_string_free;
const _: extern "C" fn(
    shell_project_path: usize,
    options_json: usize,
    callback: Option<ShellEventCallback>,
) -> usize = unitas_editor_run_shell;

const EDITOR_ABI_SCOPE_NAMES: [&str; 11] = [
    "read-project",
    "edit-content",
    "edit-scripts",
    "execute-project-code",
    "import-external-files",
    "change-packages-and-settings",
    "build",
    "sign",
    "deploy",
    "external-network",
    "review",
];

#[test]
fn editor_abi_addresses_are_pointer_sized_and_status_codes_are_four_bytes() {
    assert_eq!(
        size_of::<usize>(),
        size_of::<*mut u8>(),
        "the editor ABI address type must carry a pointer"
    );
    assert_eq!(
        align_of::<usize>(),
        align_of::<*mut u8>(),
        "the editor ABI address type must carry a pointer alignment"
    );
    assert_eq!(
        size_of::<i32>(),
        4,
        "the editor ABI status code must be four bytes"
    );
    assert_eq!(
        align_of::<i32>(),
        4,
        "the editor ABI status code must be four-byte aligned"
    );
}

#[test]
fn editor_abi_scope_names_match_the_managed_scope_table() {
    let names: Vec<&str> = Scope::ALL.iter().map(|scope| scope.as_str()).collect();
    assert_eq!(
        names, EDITOR_ABI_SCOPE_NAMES,
        "the editor ABI scope names must match the managed scope table"
    );
}

static CALLBACK_INVOCATIONS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn test_shell_callback(event_ptr: usize) -> usize {
    CALLBACK_INVOCATIONS.fetch_add(1, Ordering::SeqCst);
    let event_text = read_c_string(event_ptr)
        .and_then(Result::ok)
        .unwrap_or_default();
    let parsed = serde_json::from_str::<Value>(&event_text).unwrap_or_else(|_| json!({}));
    let event_type = parsed
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let dto = json!({
        "titleText": format!("unitas Editor — {event_type}"),
        "hierarchyRows": ["> Main Camera (100)", "  PlayerBall (131)"],
        "propertyRows": ["[Transform]", "> m_LocalPosition = {x: 0, y: 1, z: -10}"],
        "playStateText": "Play: Stopped",
        "selectedObjectId": "100",
        "selectedObjectName": "Main Camera",
        "playMode": "stopped"
    });
    allocate_c_string(&dto.to_string())
}

#[test]
fn editor_abi_run_shell_dispatches_reverse_callback_and_renders_offscreen_capture()
-> Result<(), Box<dyn std::error::Error>> {
    CALLBACK_INVOCATIONS.store(0, Ordering::SeqCst);
    let shell_project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../editor/shell-project");
    let capture_path = std::env::temp_dir().join(format!(
        "unitas_editor_abi_shell_test_{}.bmp",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&capture_path);
    let shell_ptr = allocate_c_string(&shell_project.to_string_lossy());
    let options = json!({
        "targetProjectPath": shell_project.to_string_lossy(),
        "headless": true,
        "maxFrames": 3,
        "width": 320,
        "height": 180,
        "captureOut": capture_path.to_string_lossy(),
        "events": [
            {"type": "init"},
            {"type": "select", "index": 0}
        ]
    });
    let options_ptr = allocate_c_string(&options.to_string());
    let result_ptr = unitas_editor_run_shell(shell_ptr, options_ptr, Some(test_shell_callback));
    free_c_string(shell_ptr);
    free_c_string(options_ptr);
    assert_ne!(result_ptr, 0);
    let result_text = read_c_string(result_ptr)
        .and_then(Result::ok)
        .ok_or("missing shell response string")?;
    unitas_editor_string_free(result_ptr);
    let payload: Value = serde_json::from_str(&result_text)?;
    assert_eq!(payload["ok"], json!(true));
    assert!(CALLBACK_INVOCATIONS.load(Ordering::SeqCst) >= 2);
    assert!(payload["nonBackgroundPixels"].as_u64().unwrap_or(0) > 0);
    let texts = payload["canvasTexts"]
        .as_array()
        .ok_or("missing canvasTexts array")?;
    assert!(texts.iter().any(|t| {
        t.as_str()
            .is_some_and(|s| s.contains("unitas Editor — select"))
    }));
    assert!(
        texts
            .iter()
            .any(|t| t.as_str() == Some("> Main Camera (100)"))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.as_str() == Some("  PlayerBall (131)"))
    );
    assert!(texts.iter().any(|t| t.as_str() == Some("[Transform]")));
    assert!(
        texts
            .iter()
            .any(|t| { t.as_str() == Some("> m_LocalPosition = {x: 0, y: 1, z: -10}") })
    );
    assert_eq!(
        payload["lastWorkspaceState"]["hierarchyRows"],
        json!(["> Main Camera (100)", "  PlayerBall (131)"])
    );
    assert_eq!(
        payload["lastWorkspaceState"]["propertyRows"],
        json!(["[Transform]", "> m_LocalPosition = {x: 0, y: 1, z: -10}"])
    );
    assert!(capture_path.is_file());
    let bmp_bytes = std::fs::read(&capture_path)?;
    assert!(bmp_bytes.len() > 54);
    assert_eq!(&bmp_bytes[0..2], b"BM");
    let _ = std::fs::remove_file(&capture_path);
    Ok(())
}

extern "C" fn shrinking_rows_callback(event_ptr: usize) -> usize {
    let event_text = read_c_string(event_ptr)
        .and_then(Result::ok)
        .unwrap_or_default();
    let parsed = serde_json::from_str::<Value>(&event_text).unwrap_or_else(|_| json!({}));
    let event_type = parsed.get("type").and_then(Value::as_str).unwrap_or("");
    let dto = if event_type == "init" {
        json!({
            "titleText": "unitas Editor — init",
            "hierarchyRows": ["Row0", "Row1", "Row2"],
            "propertyRows": ["Prop0", "Prop1"],
            "playStateText": "Play: Stopped"
        })
    } else {
        json!({
            "titleText": "unitas Editor — shrunk",
            "hierarchyRows": ["Row0"],
            "propertyRows": [],
            "playStateText": "Play: Stopped"
        })
    };
    allocate_c_string(&dto.to_string())
}

#[test]
fn editor_abi_run_shell_blanks_unused_hierarchy_and_property_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let shell_project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../editor/shell-project");
    let shell_ptr = allocate_c_string(&shell_project.to_string_lossy());
    let options = json!({
        "targetProjectPath": shell_project.to_string_lossy(),
        "headless": true,
        "maxFrames": 2,
        "width": 320,
        "height": 180,
        "events": [{"type": "init"}, {"type": "select", "index": 0}]
    });
    let options_ptr = allocate_c_string(&options.to_string());
    let result_ptr = unitas_editor_run_shell(shell_ptr, options_ptr, Some(shrinking_rows_callback));
    free_c_string(shell_ptr);
    free_c_string(options_ptr);
    assert_ne!(result_ptr, 0);
    let result_text = read_c_string(result_ptr)
        .and_then(Result::ok)
        .ok_or("missing shell response string")?;
    unitas_editor_string_free(result_ptr);
    let payload: Value = serde_json::from_str(&result_text)?;
    assert_eq!(payload["ok"], json!(true));
    let texts = payload["canvasTexts"]
        .as_array()
        .ok_or("missing canvasTexts array")?;
    assert!(texts.iter().any(|t| t.as_str() == Some("Row0")));
    assert!(!texts.iter().any(|t| t.as_str() == Some("Row1")));
    assert!(!texts.iter().any(|t| t.as_str() == Some("Row2")));
    assert!(!texts.iter().any(|t| t.as_str() == Some("Prop0")));
    assert!(!texts.iter().any(|t| t.as_str() == Some("Prop1")));
    Ok(())
}
