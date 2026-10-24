use super::*;

impl ConversationState {
    pub fn temporary(command: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut state = Self::new(command, cx);
        state.temporary = true;
        state
    }

    pub fn temporary_workspace(&self, key: &str) -> Option<PathBuf> {
        self.temporary
            .then(|| self.sessions.get(key).map(|s| s.info.cwd.clone()))
            .flatten()
    }

    pub(super) fn can_delete_temporary(&self, key: &str) -> bool {
        !self.draining
            && self.sessions.get(key).is_some_and(|s| {
                !s.settings_busy()
                    && !s.core_read.running()
                    && (s.instance.is_none()
                        || s.state.is_some()
                        || s.error.is_some()
                        || s.core_read.error().is_some())
            })
    }

    pub(super) fn delete_temporary(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.can_delete_temporary(key) {
            return;
        }
        self.remove_temporary(key, false, cx);
    }
    pub(super) fn remove_temporary(&mut self, key: &str, internal: bool, cx: &mut Context<Self>) {
        self.remove_temporary_with(key, internal, |path| trash_workspace(&path), cx);
    }
    fn remove_temporary_with(
        &mut self,
        key: &str,
        internal: bool,
        remove: impl FnOnce(PathBuf) -> Result<(), String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.sessions.get_mut(key) else {
            return;
        };
        let directory = s.info.cwd.clone();
        s.binding += 1;
        let directory_task = s.directory.take();
        s.reset_reads();
        s.state = None;
        s.clear_extension_ui();
        let closing = s
            .instance
            .take()
            .map(|id| gupi_pi_runtime::global(cx).update(cx, |pi, cx| pi.close(id, cx)));
        let key = key.to_owned();
        let target = key.clone();
        let task = cx.spawn(async move |owner, cx| {
            let result = async {
                if let Some(directory) = directory_task { directory.await; }
                if let Some(close) = closing
                    && close.await.is_some_and(|report| report.status.is_none())
                {
                    return Err("Pi process exit could not be confirmed".to_owned());
                }
                smol::unblock(move || remove(directory)).await
            }
            .await;
            let _ = owner.update(cx, |this, cx| {
                if let Some(s) = this.sessions.get_mut(&key) {
                    s.command.finish();
                }
                match result {
                    Ok(()) => {
                        this.sessions.remove(&key);
                        if this.selected.as_ref() == Some(&key) {
                            this.new_draft(None, cx);
                        }
                        notify_session(&key, cx);
                    }
                    Err(error) if internal => tracing::warn!(%error, "Could not discard standby conversation; retaining workspace protection"),
                    Err(error) => cx.emit(ConversationEvent::Notify { message: error.into(), error: true }),
                }
                notify_session(&key, cx);
                this.start_prewarm(cx);
            });
        });
        self.sessions.get_mut(&target).unwrap().command = SessionCommand::Deleting { task };
        notify_session(&target, cx);
    }
}

/// Only application-created directory entries are eligible. Trash the directory
/// itself; never walk links into user files referenced by a conversation.
fn validate_workspace(root: &std::path::Path, path: &std::path::Path) -> Result<bool, String> {
    if path.parent() != Some(root)
        || !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("draft-"))
    {
        return Err("Not a Gupi temporary workspace".into());
    }
    if std::fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Temporary root must not be a symlink".into());
    }
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Temporary workspace is no longer a directory".into());
    }
    Ok(true)
}

pub fn trash_workspace(path: &std::path::Path) -> Result<(), String> {
    let root = paths::temporary_dir().map_err(|e| e.to_string())?;
    if !validate_workspace(&root, path)? {
        return Ok(());
    }
    let context = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    let context = {
        use trash::macos::DeleteMethod;
        use trash::macos::TrashContextExtMacos;
        let mut context = context;
        context.set_delete_method(DeleteMethod::NsFileManager);
        context
    };
    context.delete(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ConversationState, Session, SessionInfo};
    use gpui_kit::{AppContext as _, TestAppContext};
    use std::path::PathBuf;
    use std::prelude::v1::test;
    #[gpui_kit::test]
    async fn discard_waits_for_late_directory_creation_before_removing_session(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            gupi_pi_runtime::init(cx);
        });
        let fixture = tempfile::tempdir().unwrap();
        let directory = fixture.path().join("draft-late");
        let (release, wait) = smol::channel::bounded::<()>(1);
        let owner = cx.new(|cx| ConversationState::temporary("unused-pi".into(), cx));
        owner.update(cx, |state, cx| {
            let mut session = Session::new(
                SessionInfo::new(PathBuf::new(), "late".into(), directory.clone()),
                String::new(),
            );
            session.internal = true;
            let target = directory.clone();
            session.directory = Some(cx.spawn(async move |_, _| {
                wait.recv().await.unwrap();
                std::fs::create_dir_all(target).unwrap();
            }));
            state.sessions.insert("late".into(), session);
            state.remove_temporary_with(
                "late",
                true,
                |path| std::fs::remove_dir_all(path).map_err(|e| e.to_string()),
                cx,
            );
        });
        cx.run_until_parked();
        assert!(owner.read_with(cx, |state, _| state.sessions.contains_key("late")));
        release.try_send(()).unwrap();
        cx.condition(&owner, |state, _| !state.sessions.contains_key("late"))
            .await;
        assert!(
            !directory.exists(),
            "discard cannot leave a directory created after cancellation"
        );
    }
}
