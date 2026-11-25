//! Native entry-point ABI for editor tools.
//!
//! Rust capabilities register a [`DispatchTable`] via [`install_dispatch_table`];
//! the C# MCP host discovers them through [`unitas_editor_tools_describe`] and
//! calls them through [`unitas_editor_tool_invoke`]. Both exports return
//! heap-allocated JSON strings the caller releases with
//! [`unitas_editor_string_free`]. Errors are structured JSON
//! (`{"error":{"kind","message",...}}`); no panic crosses the FFI boundary.

mod descriptor;
mod dispatch;
mod ffi;

pub use descriptor::{Scope, ToolDescriptor, ToolManifest};
pub use dispatch::{AbiError, DispatchTable, DispatchTableBuilder, ToolHandler};
pub use ffi::install_dispatch_table;
