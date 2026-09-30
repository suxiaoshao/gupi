use crate::{
    foundation::updater::{self, Driver, Event},
    state::updates,
};
use gpui_kit::{App, Global, Task};

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
            let message = crate::foundation::i18n::t(cx, "updates-install-failed");
            let worker = gpui_tokio::Tokio::spawn(cx, updater::prepare_install(directory, message));
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
                                if !matches!(commit.await, Ok(Ok(()))) {
                                    tracing::error!("update helper commit failed; the installed version is unchanged");
                                }
                                cx.update(|cx| cx.quit());
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
