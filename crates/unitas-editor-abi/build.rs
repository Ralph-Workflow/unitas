fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_string_free17h98bcb335dd4f05a5E,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_tool_invoke17h130a3d79ae79bb13E,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi27unitas_editor_string_buffer17h4af0a32083550709E,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi28unitas_editor_tools_describe17h8da96f0a500f4fd8E,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi29unitas_editor_allocate_string17he54a43b45677f578E,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi35unitas_editor_install_builtin_tools17h67df1b68450e92c7E,_unitas_editor_install_builtin_tools");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_install_builtin_tools"
    );
}
