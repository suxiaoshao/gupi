use std::{os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
    MSIDBOPEN_READONLY, MsiCloseHandle, MsiDatabaseOpenViewW, MsiEnumRelatedProductsW,
    MsiGetProductInfoW, MsiOpenDatabaseW, MsiRecordGetStringW, MsiViewExecute, MsiViewFetch,
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub(super) fn installed_culture() -> Option<String> {
    installed_culture_at(std::env::current_exe().ok()?.parent()?)
}

fn installed_culture_at(directory: &Path) -> Option<String> {
    // Same stable UpgradeCode as tauri-bundler's Gupi MSI.
    let code = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, b"Gupi.exe.app.x64");
    let code = wide(&format!("{{{code}}}"));
    let current = directory.canonicalize().ok()?;
    for ix in 0..100 {
        let mut product = [0; 39];
        if unsafe { MsiEnumRelatedProductsW(code.as_ptr(), 0, ix, product.as_mut_ptr()) } != 0 {
            break;
        }
        let property = |name: &str| {
            let mut value = vec![0; 32768];
            let mut length = value.len() as u32;
            let result = unsafe {
                MsiGetProductInfoW(
                    product.as_ptr(),
                    wide(name).as_ptr(),
                    value.as_mut_ptr(),
                    &mut length,
                )
            };
            (result == 0).then(|| String::from_utf16_lossy(&value[..length as usize]))
        };
        let Some(location) = property("InstallLocation") else {
            continue;
        };
        if Path::new(&location).canonicalize().ok().as_ref() != Some(&current) {
            continue;
        }
        let length = product.iter().position(|&value| value == 0)?;
        let product = String::from_utf16_lossy(&product[..length]);
        if let Some(culture) = product_culture(&product, property) {
            return Some(culture);
        }
    }
    None
}

fn product_culture(product: &str, property: impl Fn(&str) -> Option<String>) -> Option<String> {
    let language = property("Language")?;
    if language != "0" {
        return culture(&language).map(str::to_owned);
    }
    // Released MSIs have a neutral summary (x64;0), so both Language and
    // InstalledLanguage can be 0 despite ProductLanguage being e.g. 2052.
    // Read only the registered cache, never a download or UI-language guess.
    let package = property("LocalPackage")?;
    cached_culture(Path::new(&package), product)
}

fn culture(language: &str) -> Option<&'static str> {
    match language {
        "1033" => Some("en-US"),
        "2052" => Some("zh-CN"),
        "1028" => Some("zh-TW"),
        "1041" => Some("ja-JP"),
        "1042" => Some("ko-KR"),
        "1031" => Some("de-DE"),
        "1036" => Some("fr-FR"),
        "3082" => Some("es-ES"),
        "1046" => Some("pt-BR"),
        _ => None,
    }
}

#[derive(Default)]
struct MsiHandle(u32);

impl Drop for MsiHandle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { MsiCloseHandle(self.0) };
        }
    }
}

fn cached_culture(path: &Path, product: &str) -> Option<String> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut database = MsiHandle::default();
    if unsafe { MsiOpenDatabaseW(path.as_ptr(), MSIDBOPEN_READONLY, &mut database.0) } != 0 {
        return None;
    }
    let mut view = MsiHandle::default();
    let query = wide("SELECT `Property`, `Value` FROM `Property`");
    if unsafe { MsiDatabaseOpenViewW(database.0, query.as_ptr(), &mut view.0) } != 0
        || unsafe { MsiViewExecute(view.0, 0) } != 0
    {
        return None;
    }
    let mut code = None;
    let mut language = None;
    loop {
        let mut record = MsiHandle::default();
        match unsafe { MsiViewFetch(view.0, &mut record.0) } {
            0 => {}
            ERROR_NO_MORE_ITEMS => break,
            _ => return None,
        }
        match record_string(record.0, 1)?.as_str() {
            "ProductCode" => code = record_string(record.0, 2),
            "ProductLanguage" => language = record_string(record.0, 2),
            _ => {}
        }
    }
    if !code?.eq_ignore_ascii_case(product) {
        return None;
    }
    culture(&language?).map(str::to_owned)
}

fn record_string(record: u32, field: u32) -> Option<String> {
    let mut value = vec![0; 32768];
    let mut length = value.len() as u32;
    let result = unsafe { MsiRecordGetStringW(record, field, value.as_mut_ptr(), &mut length) };
    (result == 0).then(|| String::from_utf16_lossy(&value[..length as usize]))
}

#[cfg(test)]
mod tests;
