//! Fallible native file opening; completion means the system accepted the request.
use gpui_kit::{App, AppContext as _, Task};
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::{io, path::PathBuf};

pub(super) fn open_file(path: PathBuf, cx: &mut App) -> Task<io::Result<()>> {
    cx.background_spawn(async move {
        #[cfg(target_os = "windows")]
        {
            // A dedicated thread guarantees STA ownership independently of whichever
            // COM apartment a shared executor worker may already have initialized.
            std::thread::Builder::new()
                .spawn(move || {
                    use std::os::windows::ffi::OsStrExt;
                    use windows_sys::Win32::{
                        System::Com::{
                            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
                            CoUninitialize,
                        },
                        UI::Shell::ShellExecuteW,
                    };
                    let initialized = unsafe {
                        CoInitializeEx(
                            std::ptr::null(),
                            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
                        )
                    };
                    if initialized < 0 {
                        return Err(io::Error::other(format!(
                            "COM initialization failed: {initialized:#x}"
                        )));
                    }
                    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                    let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
                    // Success means the OS accepted the request, not that an editor loaded it.
                    let result = unsafe {
                        ShellExecuteW(
                            std::ptr::null_mut(),
                            verb.as_ptr(),
                            path.as_ptr(),
                            std::ptr::null(),
                            std::ptr::null(),
                            1,
                        )
                    } as isize;
                    unsafe {
                        CoUninitialize();
                    }
                    if result > 32 {
                        Ok(())
                    } else {
                        Err(io::Error::other(format!("ShellExecute failed: {result}")))
                    }
                })?
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("Native opener thread panicked")))
        }
        #[cfg(not(target_os = "windows"))]
        {
            #[cfg(target_os = "macos")]
            let mut command = {
                let mut command = Command::new("open");
                command.arg("--").arg(&path);
                command
            };
            #[cfg(target_os = "linux")]
            let mut command = {
                let mut command = Command::new("xdg-open");
                command.arg(&path);
                command
            };
            command
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let status = command.status()?;
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "System opener exited with {status}"
                )))
            }
        }
    })
}
