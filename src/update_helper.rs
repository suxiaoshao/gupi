#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[cfg(target_os = "windows")]
#[path = "foundation/updater/plan.rs"]
mod plan;

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = run() {
        eprintln!("Gupi update: {error}");
    }
}

#[cfg(target_os = "windows")]
fn run() -> std::io::Result<()> {
    use std::{
        io::{BufRead, Write},
        os::windows::ffi::OsStrExt,
        process::Command,
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::{
            SystemInformation::GetSystemDirectoryW,
            Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        },
        UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
    };
    let mut input = std::io::stdin().lock();
    let mut line = String::new();
    input.read_line(&mut line)?;
    let plan: plan::InstallPlan = serde_json::from_str(&line)?;
    if !plan.installer.is_absolute()
        || !plan.executable.is_absolute()
        || plan.installer.extension().and_then(|x| x.to_str()) != Some("msi")
    {
        return Err(std::io::Error::other("invalid update plan"));
    }
    let parent = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, plan.parent) };
    if parent.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
    let parent = Handle(parent);
    writeln!(std::io::stdout(), "ready")?;
    std::io::stdout().flush()?;
    line.clear();
    input.read_line(&mut line)?;
    if line.trim() != "install" {
        return Ok(());
    } // EOF/crash before graceful flush: never install.
    if unsafe { WaitForSingleObject(parent.0, 60_000) } != WAIT_OBJECT_0 {
        return Err(std::io::Error::other("Gupi did not finish exiting"));
    }
    let mut system = vec![0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(system.as_mut_ptr(), system.len() as u32) };
    if length == 0 || length as usize >= system.len() {
        return Err(std::io::Error::last_os_error());
    }
    let installer = std::path::PathBuf::from(String::from_utf16_lossy(&system[..length as usize]))
        .join("msiexec.exe");
    // MSI owns elevation and rollback. The helper stays unelevated and relaunches
    // the installed app itself, including the old version after cancellation.
    let mut destination = std::ffi::OsString::from("INSTALLDIR=");
    destination.push(
        plan.executable
            .parent()
            .ok_or_else(|| std::io::Error::other("missing install directory"))?,
    );
    let status = Command::new(installer)
        .arg("/i")
        .arg(&plan.installer)
        .arg(destination)
        .args(["/passive", "/norestart", "/L*v"])
        .arg(&plan.log)
        .status();
    let success = matches!(status.as_ref().ok().and_then(|s| s.code()), Some(0 | 3010));
    let _ = std::fs::remove_file(&plan.installer);
    let relaunched = Command::new(&plan.executable).spawn();
    if !success || relaunched.is_err() {
        if let Err(error) = &relaunched {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&plan.log)
                .and_then(|mut log| writeln!(log, "Gupi relaunch failed: {error}"));
        }
        let text: Vec<_> = std::ffi::OsStr::new(&plan.failure_message)
            .encode_wide()
            .chain([0])
            .collect();
        let title: Vec<_> = "Gupi".encode_utf16().chain([0]).collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            )
        };
    }
    relaunched?;
    Ok(())
}
