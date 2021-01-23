fn main() {
    println!("cargo:rerun-if-changed=src/wrapper_base.h");

    windows_kernel_bindgen::generate_bindings("base", |x| x.ignore_functions());
}