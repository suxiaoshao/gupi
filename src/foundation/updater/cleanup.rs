use std::{fs, io, path::Path};

const FILES: [&str; 2] = ["update.msi", "gupi-update-helper.exe"];

#[cfg(target_os = "windows")]
pub(crate) fn after_update() -> io::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(super::plan::CLEANUP_ARGUMENT)) {
        return Ok(());
    }
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid update cleanup arguments",
        )
    };
    let helper = arguments
        .next()
        .and_then(|value| value.to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid != 0 && *pid != std::process::id())
        .ok_or_else(invalid)?;
    let directory = arguments
        .next()
        .map(std::path::PathBuf::from)
        .ok_or_else(invalid)?;
    if arguments.next().is_some() {
        return Err(invalid());
    }
    validate_directory(&directory)?;
    wait_for_helper(helper)?;
    remove_directory(&directory)
}

#[cfg(target_os = "windows")]
fn wait_for_helper(pid: u32) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, WAIT_OBJECT_0},
        System::Threading::{INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if process.is_null() {
        let error = io::Error::last_os_error();
        // The helper can finish before the new process reaches this entry point.
        return if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            Ok(())
        } else {
            Err(error)
        };
    }
    let result = if unsafe { WaitForSingleObject(process, INFINITE) } == WAIT_OBJECT_0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };
    unsafe { CloseHandle(process) };
    result
}

fn validate_directory(directory: &Path) -> io::Result<()> {
    let owned_name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("gupi-update-") && name.len() > "gupi-update-".len());
    if !owned_name
        || !directory.is_absolute()
        || directory.parent().map(fs::canonicalize).transpose()?
            != Some(fs::canonicalize(std::env::temp_dir())?)
        || {
            let kind = fs::symlink_metadata(directory)?.file_type();
            kind.is_symlink() || !kind.is_dir()
        }
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a Gupi update directory",
        ));
    }
    Ok(())
}

fn remove_directory(directory: &Path) -> io::Result<()> {
    validate_directory(directory)?;
    // Refuse unexpected contents and never recurse or follow directory links.
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if !FILES.iter().any(|name| entry.file_name() == *name)
            || kind.is_symlink()
            || !kind.is_file()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected update directory contents",
            ));
        }
    }
    for name in FILES {
        match fs::remove_file(directory.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    fs::remove_dir(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_update_files_and_directory_even_when_installer_is_already_gone() {
        for installer_present in [true, false] {
            let directory = tempfile::Builder::new()
                .prefix("gupi-update-")
                .tempdir()
                .unwrap();
            fs::write(directory.path().join(FILES[1]), b"helper").unwrap();
            if installer_present {
                fs::write(directory.path().join(FILES[0]), b"installer").unwrap();
            }
            remove_directory(directory.path()).unwrap();
            assert!(!directory.path().exists());
        }
    }

    #[test]
    fn refuses_unrelated_directories_and_preserves_unexpected_files() {
        let unrelated = tempfile::tempdir().unwrap();
        fs::write(unrelated.path().join(FILES[0]), b"keep").unwrap();
        assert!(remove_directory(unrelated.path()).is_err());
        assert!(unrelated.path().join(FILES[0]).exists());

        let directory = tempfile::Builder::new()
            .prefix("gupi-update-")
            .tempdir()
            .unwrap();
        fs::write(directory.path().join(FILES[1]), b"helper").unwrap();
        fs::write(directory.path().join("unrelated.txt"), b"keep").unwrap();
        assert!(remove_directory(directory.path()).is_err());
        assert!(directory.path().join(FILES[1]).exists());
        assert!(directory.path().join("unrelated.txt").exists());

        #[cfg(unix)]
        {
            let link = tempfile::Builder::new()
                .prefix("gupi-update-")
                .tempdir()
                .unwrap();
            fs::remove_dir(link.path()).unwrap();
            std::os::unix::fs::symlink(unrelated.path(), link.path()).unwrap();
            assert!(remove_directory(link.path()).is_err());
            assert!(unrelated.path().join(FILES[0]).exists());
            fs::remove_file(link.path()).unwrap();
        }
    }
}
