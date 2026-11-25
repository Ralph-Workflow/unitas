fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_free");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tool_invoke");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_string_buffer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_tools_describe");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_allocate_string");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,-exported_symbol,_unitas_editor_install_builtin_tools"
    );
}
