pub use bindgen::*;

pub use std::path::*;

#[derive(Debug)]
pub enum Error {
    RegKeyNotFound,
    DirectoryNotFound,
}

pub enum KernelDirectoryType {
    Library,
    Include,
}

pub fn get_windows_kits_dir() -> Result<PathBuf, Error> {
    winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows Kits\Installed Roots")
        .and_then(|x| x.get_value("KitsRoot10"))
        .map(|x: String| x.into())
        .map_err(|_| Error::RegKeyNotFound)
}

pub fn get_windows_kits_km_dir(
    directory_type: KernelDirectoryType,
) -> Result<PathBuf, Error> {
    get_windows_kits_dir().and_then(|windows_kits_dir| {
        windows_kits_dir
            .join(match directory_type {
                KernelDirectoryType::Library => "Lib",
                KernelDirectoryType::Include => "Include",
            })
            .read_dir()
            .map_err(|_| Error::DirectoryNotFound)
            .and_then(|directory_content| {
                directory_content
                    .filter_map(|dir| dir.ok())
                    .map(|dir| dir.path())
                    .filter(|dir| {
                        dir.components()
                            .last()
                            .and_then(|c| c.as_os_str().to_str())
                            .map(|c| {
                                c.starts_with("10.") && dir.join("km").is_dir()
                            })
                            .unwrap_or(false)
                    })
                    .max()
                    .map_or(Err(Error::DirectoryNotFound), |x| Ok(x.join("km")))
            })
    })
}

pub fn generate_bindings(
    header: &'static str, f: fn(bindgen::Builder) -> bindgen::Builder,
) {
    let out_path = PathBuf::from(
        std::env::var_os("OUT_DIR").expect("OUT_DIR environment not set?"),
    );
    let windows_kits_km_include_dir =
        get_windows_kits_km_dir(KernelDirectoryType::Include)
            .expect("Failed to retrieve windows kits km dir");
    let bindings = bindgen::Builder::default()
        .header(format!("src/wrapper_{}.h", header))
        .use_core()
        .derive_debug(false)
        .layout_tests(false)
        .ctypes_prefix("cty")
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .clang_arg(format!(
            "-I{}",
            windows_kits_km_include_dir
                .to_str()
                .expect("UTF-8 error on include path")
        ))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks));

    f(bindings)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join(format!("bindings_{}.rs", header)))
        .expect("Unable to write binding file");
}
