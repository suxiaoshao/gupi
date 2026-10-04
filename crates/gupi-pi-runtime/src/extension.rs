//! Bundled Pi commands use public extension APIs and standard RPC UI events.
use pi_rpc::{Error, LaunchOptions};

pub const TREE_COMMAND: &str = "gupi-continue";

pub(super) fn install(options: &mut LaunchOptions) -> Result<tempfile::TempDir, Error> {
    let directory = tempfile::tempdir().map_err(|error| Error::Io(error.to_string()))?;
    let path = directory.path().join("gupi.ts");
    std::fs::write(&path, include_str!("extension.ts"))
        .map_err(|error| Error::Io(error.to_string()))?;
    options.args.push("--extension".into());
    options.args.push(path.into_os_string());
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pi_rpc::{Client, protocol::Prompt};
    use serde_json::json;
    use std::time::Duration;

    /// Uses installed Pi without network requests, credentials, or user sessions.
    #[tokio::test]
    #[ignore = "requires installed Pi CLI"]
    async fn installed_pi_navigates_without_forking_or_starting_a_turn() {
        let root = tempfile::tempdir().unwrap();
        let session = root.path().join("history.jsonl");
        let timestamp = "2026-10-04T00:00:00.000Z";
        let mut entries = vec![json!({
            "type": "session", "version": 3,
            "id": "2a0a6ae6-dd21-489e-88f5-99662ea85b95",
            "timestamp": timestamp, "cwd": root.path()
        })];
        for (id, parent, text) in [
            ("u1", None, "first question"),
            ("u2", Some("u1"), "second question"),
        ] {
            entries.push(json!({
                "type": "message", "id": id, "parentId": parent,
                "timestamp": timestamp,
                "message": {"role": "user", "content": [{"type": "text", "text": text}], "timestamp": 1}
            }));
        }
        entries.push(json!({"type":"custom", "id":"c1", "parentId":"u2", "timestamp":timestamp, "customType":"fixture"}));
        std::fs::write(
            &session,
            entries.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        let cancel = root.path().join("cancel.ts");
        std::fs::write(&cancel, r#"export default function(pi) {
            pi.on("session_before_tree", (event) => event.preparation.targetId === "u2" ? { cancel: true } : undefined);
        }"#).unwrap();
        let mut options = LaunchOptions::new("pi", root.path());
        options.args = vec![
            "--offline".into(),
            "--no-extensions".into(),
            "--no-skills".into(),
            "--no-prompt-templates".into(),
            "--session".into(),
            session.clone().into_os_string(),
            "--extension".into(),
            cancel.into_os_string(),
        ];
        options.env.push((
            "PI_CODING_AGENT_DIR".into(),
            root.path().join("agent").into_os_string(),
        ));
        let _extension = install(&mut options).unwrap();
        let (client, mut events) = Client::spawn(options).await.unwrap();
        client.ready().await.unwrap();
        assert!(
            client
                .get_commands()
                .await
                .unwrap()
                .commands
                .iter()
                .any(|c| c.name == TREE_COMMAND)
        );
        let before = client.get_state().await.unwrap();
        for (target, leaf, editor, error) in [
            ("u1", None, Some("first question"), false),
            ("c1", Some("c1"), Some(""), false),
            ("u2", Some("c1"), None, false), // another extension cancels
            ("c1", Some("c1"), None, false), // current leaf is a no-op
            ("missing", Some("c1"), None, true),
        ] {
            client
                .prompt(Prompt::new(format!("/{TREE_COMMAND} {target}")))
                .await
                .unwrap();
            assert_eq!(client.get_entries().await.unwrap().leaf_id.as_deref(), leaf);
            let mut actual_editor = None;
            let mut actual_error = false;
            while let Ok(Some(event)) =
                tokio::time::timeout(Duration::from_millis(20), events.recv()).await
            {
                let raw = event.raw();
                assert_ne!(raw["type"], "agent_start");
                if raw["method"] == "set_editor_text" {
                    actual_editor = raw["text"].as_str().map(str::to_owned);
                }
                actual_error |= raw["type"] == "extension_error";
            }
            assert_eq!(actual_editor.as_deref(), editor);
            assert_eq!(actual_error, error);
        }
        let after = client.get_state().await.unwrap();
        assert_eq!(before.session_id, after.session_id);
        assert_eq!(before.session_file, after.session_file);
        drop(client);
        drop(events);
    }
}
