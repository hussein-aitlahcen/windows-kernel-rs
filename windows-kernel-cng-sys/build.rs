fn main() {
    println!("cargo:rustc-link-lib=cng");
    println!("cargo:rerun-if-changed=src/wrapper.h");
    windows_kernel_bindgen::generate_bindings("bcrypt", |x| 
        x
        .whitelist_function("BCRYPT.*")
        .whitelist_function("BCrypt.*")
        .whitelist_type("_BCrypt.*")
        .whitelist_type("BCrypt.*")
        .whitelist_type("PBCrypt.*")
        .whitelist_type("_BCRYPT.*")
        .whitelist_type("BCRYPT.*")
        .whitelist_type("PBCRYPT.*")
        .whitelist_type("_CRYPT.*")
        .whitelist_type("CRYPT.*")
        .whitelist_type("PCRYPT.*")
        .whitelist_type("__BCRYPT.*")
        .whitelist_type("__BCRYPT.*")
        .whitelist_type("ECC_.*")
        .whitelist_type("HASHALGORITHM.*")
        .whitelist_type("DSA.*")
        .whitelist_type("DSA.*")
        .whitelist_recursively(false)
    );
}
