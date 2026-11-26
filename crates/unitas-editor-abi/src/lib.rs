mod asset_tools;
mod build_tools;
mod compat_tools;
mod descriptor;
mod diagnostics_tools;
mod dispatch;
mod ffi;

pub use descriptor::{Scope, ToolDescriptor, ToolManifest};
pub use dispatch::{AbiError, DispatchTable, DispatchTableBuilder, ToolHandler};
pub use ffi::{
    allocate_c_string, builtin_dispatch_table, ensure_default_dispatch_table,
    install_dispatch_table, unitas_editor_allocate_string, unitas_editor_install_builtin_tools,
    unitas_editor_string_buffer, unitas_editor_string_free, unitas_editor_tool_invoke,
    unitas_editor_tools_describe,
};
