fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_string_free17h26ff7dd38ad1510eE,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_tool_invoke17h2f964f05f1e4f00fE,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi27unitas_editor_string_buffer17h6b52c72e003c9a55E,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi28unitas_editor_tools_describe17h23155562b671a84dE,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi29unitas_editor_allocate_string17hfe6269f8d9f2fdd1E,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi35unitas_editor_install_builtin_tools17ha7d56c3ad6279937E,_unitas_editor_install_builtin_tools");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_install_builtin_tools"
    );
}
