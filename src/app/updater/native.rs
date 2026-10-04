use gpui_kit::App;
use gpui_kit::Global;
use gpui_kit::Task;
use gupi_updates::updater;
use gupi_updates::updater::Driver;
use gupi_updates::updater::Event;
use gupi_updates::updates;

struct Updater {
    driver: Option<Driver>,
    _events: Task<()>,
    #[cfg(target_os = "windows")]
    prepare: Option<Task<()>>,
}
impl Global for Updater {}

pub(crate) fn init(cx: &mut App) {
    if cx.has_global::<Updater>() {
        return;
    }
    let (sender, receiver) = smol::channel::unbounded();
    let driver = Driver::new(sender)
        .map_err(|error| tracing::debug!(%error, "native updater unavailable"))
        .ok();
    let events = cx.spawn(async move |cx| {
        while let Ok(event) = receiver.recv().await {
            cx.update(|cx| handle(event, cx));
        }
    });
    cx.set_global(Updater {
        driver,
        _events: events,
        #[cfg(target_os = "windows")]
        prepare: None,
    });
}

pub(crate) fn available(cx: &App) -> bool {
    cx.try_global::<Updater>()
        .is_some_and(|state| state.driver.is_some())
}

pub(crate) fn install(cx: &mut App) {
    if !available(cx) {
        return;
    }
    let owner = updates::get(cx);
    if owner.read(cx).is_installing() {
        #[cfg(target_os = "windows")]
        if cx.global::<Updater>().prepare.is_some() {
            return;
        }
        cx.global::<Updater>().driver.as_ref().unwrap().show();
        return;
    }
    let Some(main) = cx.try_global::<super::super::MainWindow>() else {
        return;
    };
    let config = main.view.read(cx).config.clone();
    if config.read(cx).busy(cx) {
        return;
    }
    let Some(release) = owner.update(cx, |owner, cx| owner.start_install(cx)) else {
        return;
    };
    let driver = cx.global::<Updater>().driver.as_ref().unwrap();
    let url = updater::feed_url(&release.url, &driver.feed_name());
    if let Err(error) = driver.install(&url) {
        tracing::error!(%error, "start native update failed");
        owner.update(cx, |owner, cx| owner.finish_install(true, cx));
    }
}

fn handle(event: Event, cx: &mut App) {
    match event {
        Event::Skipped => {
            if let Some(main) = cx.try_global::<super::super::MainWindow>() {
                let config = main.view.read(cx).config.clone();
                config.update(cx, |owner, cx| owner.skip_update(cx));
            }
        }
        #[cfg(target_os = "macos")]
        Event::Ready => super::super::quit_then(cx, |cx| {
            // Sparkle resumes its installer and terminates/relaunches the app.
            cx.global::<Updater>().driver.as_ref().unwrap().resume();
        }),
        Event::Finished | Event::Failed => {
            #[cfg(target_os = "windows")]
            if cx.global::<Updater>().prepare.is_some() {
                return;
            }
            updates::get(cx).update(cx, |owner, cx| {
                owner.finish_install(matches!(event, Event::Failed), cx)
            });
        }
        #[cfg(target_os = "windows")]
        Event::Installer(directory) => {
            if cx.global::<Updater>().prepare.is_some() {
                return;
            }
            let message = gupi_settings::i18n::t(cx, "updates-install-failed");
            let log = gupi_resources::paths::log_dir().map(|p| p.join("update-install.log"));
            let worker = gpui_tokio::Tokio::spawn(cx, async move {
                updater::prepare_install(directory, message, log.map_err(|e| e.to_string())?).await
            });
            let task = cx.spawn(async move |cx| {
                let result = worker
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|result| result);
                cx.update(|cx| {
                    cx.global_mut::<Updater>().prepare = None;
                    match result {
                        Err(error) => {
                            tracing::error!(%error, "prepare update installation failed");
                            updates::get(cx).update(cx, |owner, cx| owner.finish_install(true, cx));
                        }
                        Ok(prepared) => super::super::quit_then(cx, move |cx| {
                            let commit = gpui_tokio::Tokio::spawn(cx, prepared.commit());
                            let task = cx.spawn(async move |cx| {
                                let result = commit
                                    .await
                                    .map_err(|error| error.to_string())
                                    .and_then(|result| result);
                                cx.update(|cx| finish_handoff(result, cx));
                            });
                            cx.global_mut::<Updater>().prepare = Some(task);
                        }),
                    }
                });
            });
            cx.global_mut::<Updater>().prepare = Some(task);
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn finish_handoff(result: Result<(), String>, cx: &mut App) {
    match result {
        Ok(()) => cx.quit(),
        Err(error) => {
            tracing::error!(%error, "update helper commit failed; restarting the installed version");
            // Managed shutdown already stopped the application's services. Restart
            // through GPUI, which waits for this process to exit on Windows.
            cx.restart();
        }
    }
}

#[cfg(test)]
mod tests {
    #[gpui_kit::test]
    async fn failed_handoff_restarts_the_existing_application(cx: &mut gpui_kit::TestAppContext) {
        let restarted = cx.expect_restart();
        cx.update(|cx| super::finish_handoff(Err("helper pipe closed".into()), cx));
        let (path, arguments) = restarted
            .await
            .expect("failed handoff must request a restart");
        assert!(path.is_none(), "use the current installed executable");
        assert!(arguments.is_empty());
    }
}
