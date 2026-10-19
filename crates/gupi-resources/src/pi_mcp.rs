//! Official MCP CLI operations. Raw output and credentials never leave this module.
mod process;
use crate::pi_settings::Scope;
use serde_json::Value;
use std::{
    collections::HashSet, ffi::OsString, future::Future, pin::Pin, sync::Arc, time::Duration,
};
/// The environment may still be loading (e.g. the macOS login-shell PATH); it
/// is awaited inside the operation's cancellation and deadline.
pub type Variables = Pin<Box<dyn Future<Output = Vec<(OsString, OsString)>> + Send>>;
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    Ineligible,
    UntrustedProject,
    UnsupportedName,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ServerScope {
    Global,
    Project,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Connecting,
    Connected,
    Disconnected,
    NeedsAuth,
    Failed,
    Closed,
    Disabled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    name: String,
    scope: ServerScope,
    enabled: bool,
    state: State,
    tools: usize,
    resources: bool,
}
impl Server {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn scope(&self) -> ServerScope {
        self.scope
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn tools(&self) -> usize {
        self.tools
    }
    pub fn has_resources(&self) -> bool {
        self.resources
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    servers: Vec<Server>,
    configuration_errors: bool,
    untrusted_project: bool,
}
impl Report {
    pub fn servers(&self) -> &[Server] {
        &self.servers
    }
    pub fn has_configuration_errors(&self) -> bool {
        self.configuration_errors
    }
    pub fn has_untrusted_project(&self) -> bool {
        self.untrusted_project
    }
    pub fn server(&self, scope: ServerScope, name: &str) -> Option<&Server> {
        self.servers
            .iter()
            .find(|server| server.scope == scope && server.name == name)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Checked(Report),
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Check,
    Login(String),
    Logout(String),
}
impl Action {
    fn deadline(&self) -> Duration {
        Duration::from_secs(match self {
            Self::Check => 60,
            Self::Login(_) => 420,
            Self::Logout(_) => 30,
        })
    }
    fn output_limit(&self) -> usize {
        if matches!(self, Self::Check) {
            2 * 1024 * 1024
        } else {
            64 * 1024
        }
    }
    fn arguments(&self) -> Option<Vec<&str>> {
        match self {
            Self::Check => Some(vec!["mcp", "list", "--json"]),
            Self::Login(name) if addressable_name(name) => {
                Some(vec!["mcp", "login", "--timeout", "300", "--", name])
            }
            Self::Logout(name) if addressable_name(name) => Some(vec!["mcp", "logout", "--", name]),
            _ => None,
        }
    }
}
fn server_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
/// Pi's MCP dispatcher intercepts these help tokens even after `--`.
pub fn addressable_name(name: &str) -> bool {
    server_name(name) && !matches!(name, "--help" | "-h")
}

fn parse(code: Option<i32>, bytes: &[u8]) -> Option<Report> {
    if !matches!(code, Some(0 | 1)) {
        return None;
    }
    let root: Value = serde_json::from_slice(bytes).ok()?;
    let errors = root.get("errors")?.as_array()?;
    if !errors.iter().all(Value::is_string) {
        return None;
    }
    let untrusted_project = match root.get("note") {
        None => false,
        Some(Value::String(_)) => true,
        _ => return None,
    };
    let mut identities = HashSet::new();
    let servers = root
        .get("servers")?
        .as_array()?
        .iter()
        .map(|raw| {
            let name = raw.get("name")?.as_str()?;
            if !server_name(name) {
                return None;
            }
            let scope = match raw.get("scope")?.as_str()? {
                "global" => ServerScope::Global,
                "project" => ServerScope::Project,
                _ => return None,
            };
            if !identities.insert((scope, name)) {
                return None;
            }
            let enabled = raw.get("enabled")?.as_bool()?;
            let state = match raw.get("state")?.as_str()? {
                "connecting" => State::Connecting,
                "connected" => State::Connected,
                "disconnected" => State::Disconnected,
                "needs-auth" => State::NeedsAuth,
                "failed" => State::Failed,
                "closed" => State::Closed,
                "disabled" => State::Disabled,
                _ => return None,
            };
            let tools = raw.get("tools")?.as_array()?;
            if !tools.iter().all(Value::is_string) {
                return None;
            }
            let resources = match raw.get("resources") {
                None => false,
                Some(Value::Bool(value)) => *value,
                _ => return None,
            };
            Some(Server {
                name: name.into(),
                scope,
                enabled,
                state,
                tools: tools.len(),
                resources,
            })
        })
        .collect::<Option<_>>()?;
    Some(Report {
        servers,
        configuration_errors: !errors.is_empty(),
        untrusted_project,
    })
}

/// A per-page scope may own child cancellation signals for individual operations.
/// Cancellation only signals; the detached operation worker performs stop + reap.
#[derive(Clone)]
pub struct Cancellation {
    sender: watch::Sender<bool>,
    parent: Option<Arc<Self>>,
}
impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}
impl Cancellation {
    pub fn new() -> Self {
        Self {
            sender: watch::channel(false).0,
            parent: None,
        }
    }
    pub fn child(&self) -> Self {
        Self {
            parent: Some(Arc::new(self.clone())),
            ..Self::new()
        }
    }
    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
            || self
                .parent
                .as_ref()
                .is_some_and(|parent| parent.is_cancelled())
    }
    fn cancelled(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let own = async {
                let mut receiver = self.sender.subscribe();
                while !*receiver.borrow_and_update() {
                    if receiver.changed().await.is_err() {
                        break;
                    }
                }
            };
            match &self.parent {
                Some(parent) => tokio::select! { _ = own => {}, _ = parent.cancelled() => {} },
                None => own.await,
            }
        })
    }
}
struct CancelOnDrop(Cancellation);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Keeps cleanup alive even if the UI/awaiting future disappears. The inner
/// Tokio task is detached on dropped JoinHandle, observes cancellation, and reaps.
pub async fn run(
    command: OsString,
    action: Action,
    scope: Scope,
    variables: Variables,
    cancellation: Cancellation,
) -> Outcome {
    let deadline = action.deadline();
    run_with(command, action, scope, variables, cancellation, deadline).await
}
async fn run_with(
    command: OsString,
    action: Action,
    scope: Scope,
    variables: Variables,
    cancellation: Cancellation,
    deadline: Duration,
) -> Outcome {
    let cancel = cancellation.child();
    let _guard = CancelOnDrop(cancel.clone());
    tokio::spawn(process::execute(
        command, action, scope, variables, cancel, deadline,
    ))
    .await
    .unwrap_or(Outcome::Failed)
}

#[cfg(test)]
mod tests;
