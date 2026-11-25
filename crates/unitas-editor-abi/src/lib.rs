mod descriptor;
mod dispatch;
mod ffi;

pub use descriptor::{Scope, ToolDescriptor, ToolManifest};
pub use dispatch::{AbiError, DispatchTable, DispatchTableBuilder, ToolHandler};
pub use ffi::{
    install_dispatch_table, unitas_editor_string_free, unitas_editor_tool_invoke,
    unitas_editor_tools_describe,
};
