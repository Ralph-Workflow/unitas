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

use unitas_editor_abi::{
    Scope, unitas_editor_allocate_string, unitas_editor_install_builtin_tools,
    unitas_editor_string_buffer, unitas_editor_string_free, unitas_editor_tool_invoke,
    unitas_editor_tools_describe,
};

const _: extern "C" fn(capacity: usize) -> usize = unitas_editor_string_buffer;
const _: extern "C" fn(address: usize, length: usize) -> usize = unitas_editor_allocate_string;
const _: extern "C" fn() -> i32 = unitas_editor_install_builtin_tools;
const _: extern "C" fn() -> usize = unitas_editor_tools_describe;
const _: extern "C" fn(name: usize, args_json: usize) -> usize = unitas_editor_tool_invoke;
const _: extern "C" fn(ptr: usize) = unitas_editor_string_free;

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
