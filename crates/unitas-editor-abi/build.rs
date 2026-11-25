fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_string_free17h30e802f55a99546aE,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi25unitas_editor_tool_invoke17h70cbc53f42dc3df8E,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi27unitas_editor_string_buffer17h3b25e4a00a50bcd8E,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi28unitas_editor_tools_describe17h5d31e89023b2a78cE,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi29unitas_editor_allocate_string17h32791d5c096237b9E,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_allocate_string");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-alias,__ZN17unitas_editor_abi3ffi35unitas_editor_install_builtin_tools17h0ab760c46ba0dda5E,_unitas_editor_install_builtin_tools");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_install_builtin_tools"
    );
}
