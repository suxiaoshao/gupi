#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod components;
mod features;

#[cfg(all(feature = "performance", not(debug_assertions)))]
compile_error!(
    "performance tracing is development-only; use --profile performance, never --release"
);

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = gupi_updates::updater::after_update() {
        eprintln!("Gupi update cleanup: {error}");
    }
    #[cfg(target_os = "macos")]
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--gupi-print-shell-path"))
    {
        if gupi_pi_runtime::print_path().is_err() {
            std::process::exit(1);
        }
        return;
    }
    #[cfg(feature = "performance")]
    let _performance = app::performance::start();
    app::run();
}
