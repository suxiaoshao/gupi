use gpui_form::Form;
use gpui_form::FormSchema;
use gpui_form::FormVersion;
use gpui_kit::*;
use gpui_operation::Complete;
use gpui_operation::Load;
use gpui_operation::Refresh;
use gpui_operation::Repair;
use gpui_operation::Transition;
use gpui_operation::repair;
use gpui_store::Store;
use gupi_resources::paths;
use gupi_resources::persistence;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
    #[default]
    System,
    English,
    Chinese,
    TraditionalChinese,
    Japanese,
    Korean,
    German,
    French,
    Spanish,
    BrazilianPortuguese,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FormSchema)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct AppConfig {
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub keybindings: crate::keybindings::Overrides,
    pub shortcuts: crate::shortcuts::Shortcuts,
    pub pi_command: Option<String>,
    pub theme: ThemeMode,
    pub icon_theme: crate::icons::IconTheme,
    pub light_theme: Option<String>,
    pub dark_theme: Option<String>,
    pub language: AppLanguage,
    pub notifications: crate::notifications::Preferences,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_check_updates: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_update: Option<String>,
}
impl AppConfig {
    pub fn checks_updates_automatically(&self) -> bool {
        self.auto_check_updates.unwrap_or(cfg!(feature = "bundled"))
    }
    pub fn pi_executable(&self) -> PathBuf {
        self.pi_command.as_deref().unwrap_or("pi").into()
    }
    pub fn normalized(mut self) -> Result<Self, String> {
        for (id, binding) in &self.keybindings {
            if !crate::keybindings::COMMANDS.iter().any(|c| c.id == id) {
                return Err("error-config-validation".into());
            }
            crate::keybindings::syntax(binding).map_err(str::to_owned)?;
        }
        self.shortcuts.validate()?;
        let globals = std::iter::once(&self.shortcuts.launcher)
            .chain(
                self.shortcuts
                    .tasks
                    .iter()
                    .filter(|t| t.enabled)
                    .map(|t| &t.binding),
            )
            .filter(|s| !s.is_empty())
            .map(|s| crate::shortcuts::system_binding(s))
            .collect::<Result<Vec<_>, _>>()?;
        for command in crate::keybindings::COMMANDS {
            let binding = command.value(&self.keybindings);
            if !binding.is_empty()
                && let Ok(binding) = crate::shortcuts::system_binding(binding)
                && globals.contains(&binding)
            {
                return Err("settings-key-conflict".into());
            }
        }

        self.pi_command = PiSettings {
            command: self.pi_command,
        }
        .normalized()?
        .command;
        Ok(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, FormSchema)]
#[non_exhaustive]
pub struct PiSettings {
    pub command: Option<String>,
}
impl PiSettings {
    pub fn new(command: Option<String>) -> Self {
        Self { command }
    }

    pub fn normalized(mut self) -> Result<Self, String> {
        self.command = self
            .command
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        if let Some(command) = &self.command {
            let path = std::path::Path::new(command);
            if command.chars().any(char::is_control)
                || (!path.is_absolute()
                    && (command.contains(char::is_whitespace) || path.components().count() != 1))
            {
                return Err("error-config-validation".into());
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub enum PreferenceChange {
    Keybinding(String, Option<String>),
    ResetKeybindings,
    Notifications(crate::notifications::Preferences),
    AutoCheckUpdates(bool),
    SkipUpdate(String),
    Shortcuts(crate::shortcuts::Shortcuts),
    Language(AppLanguage),
    Theme(ThemeMode),
    IconTheme(crate::icons::IconTheme),
    LightTheme(Option<String>),
    DarkTheme(Option<String>),
}
impl PreferenceChange {
    fn apply(self, config: &mut AppConfig) {
        match self {
            Self::AutoCheckUpdates(value) => config.auto_check_updates = Some(value),
            Self::SkipUpdate(value) => config.skipped_update = Some(value),
            Self::Notifications(value) => config.notifications = value,
            Self::ResetKeybindings => {
                config.keybindings.clear();
                config.shortcuts.clear_bindings();
            }
            Self::Shortcuts(value) => config.shortcuts = value,
            Self::Keybinding(id, binding) => match binding {
                Some(binding) => {
                    config.keybindings.insert(id, binding);
                }
                None => {
                    config.keybindings.remove(&id);
                }
            },
            Self::Language(value) => config.language = value,
            Self::Theme(value) => config.theme = value,
            Self::IconTheme(value) => config.icon_theme = value,
            Self::LightTheme(value) => config.light_theme = value,
            Self::DarkTheme(value) => config.dark_theme = value,
        }
    }
}
#[derive(Clone, Debug)]
pub enum ConfigContents {
    Missing,
    Configured(AppConfig),
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ConfigData {
    pub path: PathBuf,
    pub contents: ConfigContents,
    pub backup: Option<PathBuf>,
}
impl ConfigData {
    pub fn new(path: PathBuf, contents: ConfigContents) -> Self {
        Self {
            path,
            contents,
            backup: None,
        }
    }

    pub fn configured(&self) -> Option<&AppConfig> {
        match &self.contents {
            ConfigContents::Missing => None,
            ConfigContents::Configured(value) => Some(value),
        }
    }
}
struct PendingConfig {
    value: AppConfig,
    baseline: Option<AppConfig>,
    version: Option<FormVersion>,
}
#[derive(Debug, thiserror::Error)]
pub enum ConfigProblem {
    #[error("configuration read failed: {0}")]
    Read(String),
    #[error("configuration parse failed: {0}")]
    Parse(String),
    #[error("invalid configuration: {0}")]
    Validation(String),
    #[error("configuration write failed: {failure}")]
    Write {
        failure: std::io::Error,
        backup: Option<PathBuf>,
    },
}
impl ConfigProblem {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Read(_) => "error-config-read",
            Self::Parse(_) => "error-config-parse",
            Self::Validation(_) => "error-config-validation",
            Self::Write { .. } => "error-config-write",
        }
    }
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Write { .. })
    }
    pub fn backup(&self) -> Option<&PathBuf> {
        match self {
            Self::Write { backup, .. } => backup.as_ref(),
            Self::Read(_) | Self::Parse(_) | Self::Validation(_) => None,
        }
    }
}
#[derive(Clone, Debug)]
pub enum ConfigRepair {
    Reload,
    SaveDraft,
    SavePi,
    SavePreference(PreferenceChange),
    WriteCommitted,
    BackupAndReset,
}
pub type ConfigOperation = repair::Operation<ConfigData, ConfigProblem, ConfigRepair, Task<()>>;
pub type ConfigStore = Store<ConfigOperation>;

pub fn read_config(path: PathBuf, require_configured: bool) -> Result<ConfigData, ConfigProblem> {
    let bytes = persistence::read(&path).map_err(|e| ConfigProblem::Read(e.to_string()))?;
    let contents = match &bytes {
        None if require_configured => {
            return Err(ConfigProblem::Read("file removed".into()));
        }
        None => ConfigContents::Missing,
        Some(bytes) => {
            let text =
                std::str::from_utf8(bytes).map_err(|e| ConfigProblem::Parse(e.to_string()))?;
            let value: AppConfig = toml::from_str(text)
                .map_err(|_| ConfigProblem::Parse("invalid TOML or unsupported fields".into()))?;
            ConfigContents::Configured(value.normalized().map_err(ConfigProblem::Validation)?)
        }
    };
    Ok(ConfigData {
        path,
        contents,
        backup: None,
    })
}
fn write_config(
    path: PathBuf,
    pending: PendingConfig,
    reset: bool,
    reset_keybindings: bool,
    mut backup: Option<PathBuf>,
) -> Result<ConfigData, ConfigProblem> {
    let mut value = if reset {
        if let Some(bytes) =
            persistence::read(&path).map_err(|e| ConfigProblem::Read(e.to_string()))?
        {
            backup = Some(persistence::backup(&path, &bytes).map_err(|failure| {
                ConfigProblem::Write {
                    failure,
                    backup: backup.clone(),
                }
            })?);
        }
        pending.value
    } else {
        let mut latest = read_config(path.clone(), false)?
            .configured()
            .cloned()
            .unwrap_or_default();
        if let Some(baseline) = pending.baseline {
            let value = pending.value;
            if value.pi_command != baseline.pi_command {
                latest.pi_command = value.pi_command;
            }
            if value.theme != baseline.theme {
                latest.theme = value.theme;
            }
            if value.icon_theme != baseline.icon_theme {
                latest.icon_theme = value.icon_theme;
            }
            if value.light_theme != baseline.light_theme {
                latest.light_theme = value.light_theme;
            }
            if value.dark_theme != baseline.dark_theme {
                latest.dark_theme = value.dark_theme;
            }
            if value.notifications != baseline.notifications {
                latest.notifications = value.notifications;
            }
            if value.auto_check_updates != baseline.auto_check_updates {
                latest.auto_check_updates = value.auto_check_updates;
            }
            if value.skipped_update != baseline.skipped_update {
                latest.skipped_update = value.skipped_update;
            }
            if value.shortcuts != baseline.shortcuts {
                latest.shortcuts = value.shortcuts;
            }
            if value.language != baseline.language {
                latest.language = value.language;
            }
            for id in value.keybindings.keys().chain(baseline.keybindings.keys()) {
                if value.keybindings.get(id) != baseline.keybindings.get(id) {
                    if let Some(binding) = value.keybindings.get(id) {
                        latest.keybindings.insert(id.clone(), binding.clone());
                    } else {
                        latest.keybindings.remove(id);
                    }
                }
            }
            latest
        } else {
            // The explicit "write applied settings" action writes all applied values.
            pending.value
        }
    };
    if reset_keybindings {
        value.keybindings.clear();
        value.shortcuts.clear_bindings();
    }
    let value = value.normalized().map_err(ConfigProblem::Validation)?;
    let bytes =
        toml::to_string_pretty(&value).map_err(|e| ConfigProblem::Validation(e.to_string()))?;
    persistence::write_atomic(&path, bytes.as_bytes()).map_err(|failure| ConfigProblem::Write {
        failure,
        backup: backup.clone(),
    })?;
    Ok(ConfigData {
        path,
        contents: ConfigContents::Configured(value),
        backup,
    })
}

/// Read-only configuration operation projection. Mutation stays in ConfigController.
#[derive(Clone)]
pub struct ConfigReader(ConfigStore);
impl ConfigReader {
    pub fn read<R>(&self, cx: &impl AppContext, read: impl FnOnce(&ConfigOperation) -> R) -> R {
        self.0.read(cx, read)
    }
    pub fn observe_in<Owner: 'static>(
        &self,
        cx: &mut Context<Owner>,
        window: &mut Window,
        observe: impl FnMut(&mut Owner, &ConfigOperation, &mut Window, &mut Context<Owner>) + 'static,
    ) -> Subscription {
        self.0.observe_in(cx, window, observe)
    }
}

pub struct ConfigController {
    store: ConfigStore,
    form: WeakEntity<Form<AppConfig>>,
    pi_form: Entity<Form<PiSettings>>,
    draining: bool,
    _admission_subscription: Subscription,
    path: Result<PathBuf, String>,
}

impl ConfigController {
    pub fn configuration(&self) -> ConfigReader {
        ConfigReader(self.store.clone())
    }
    pub fn pi_form(&self) -> &Entity<Form<PiSettings>> {
        &self.pi_form
    }
    pub fn begin_shutdown(&mut self) {
        self.draining = true;
    }
    #[cfg(feature = "test-support")]
    pub fn settle_for_test(&self, data: ConfigData, cx: &mut App) {
        self.store.update(cx, |op| {
            op.transition(gpui_operation::Cancel);
            op.transition(gpui_operation::Settle(Ok(data)));
        });
    }

    pub fn new(form: &Entity<Form<AppConfig>>, cx: &mut Context<Self>) -> Self {
        Self::at_path(
            form,
            paths::config_dir()
                .map(|p| p.join("config.toml"))
                .map_err(|e| e.to_string()),
            cx,
        )
    }
    pub fn at_path(
        form: &Entity<Form<AppConfig>>,
        path: Result<PathBuf, String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let admission_subscription = crate::host::observe_admission(&cx.entity(), cx);
        Self {
            store: Store::new(cx, ConfigOperation::new()),
            form: form.downgrade(),
            pi_form: cx.new(|_| Form::new(PiSettings::default())),
            draining: false,
            _admission_subscription: admission_subscription,
            path,
        }
    }
    pub fn busy(&self, cx: &App) -> bool {
        self.draining || self.is_running(cx) || crate::host::writes_blocked(cx)
    }
    pub fn is_running(&self, cx: &App) -> bool {
        self.store.read(cx, |op| op.is_running())
    }
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref().ok()
    }
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        if self.busy(cx) {
            return;
        }
        self.start(ConfigRepair::Reload, None, cx);
    }
    pub fn is_onboarding(&self, cx: &App) -> bool {
        self.store.read(cx, |op| {
            op.data().is_some_and(|data| data.configured().is_none())
        })
    }
    pub fn preferences(&self, cx: &App) -> AppConfig {
        self.store
            .read(cx, |op| op.data().and_then(ConfigData::configured).cloned())
            .unwrap_or_else(|| {
                self.form
                    .upgrade()
                    .map(|form| AppConfig::ROOT.get(&form, cx))
                    .unwrap_or_default()
            })
    }
    pub fn set_preference(&mut self, change: PreferenceChange, cx: &mut Context<Self>) {
        if self.busy(cx) {
            return;
        }
        self.save_preference(change, cx);
    }
    pub fn skip_update(&mut self, cx: &mut Context<Self>) {
        let Some(version) = crate::host::skipped_version(cx) else {
            return;
        };
        // The native update session excludes other configuration operations. Its own
        // skip action can save while the session still keeps settings disabled.
        self.save_preference(PreferenceChange::SkipUpdate(version), cx);
    }
    fn save_preference(&mut self, change: PreferenceChange, cx: &mut Context<Self>) {
        if self.draining || self.is_running(cx) {
            return;
        }
        if self.is_onboarding(cx) {
            if let Some(form) = self.form.upgrade() {
                let mut draft = AppConfig::ROOT.get(&form, cx);
                change.apply(&mut draft);
                AppConfig::ROOT.set(&form, draft, cx);
            }
            return;
        }
        let baseline = self.preferences(cx);
        let mut value = baseline.clone();
        change.clone().apply(&mut value);
        if value == baseline {
            return;
        }
        self.start(
            ConfigRepair::SavePreference(change),
            Some(PendingConfig {
                value,
                baseline: Some(baseline),
                version: None,
            }),
            cx,
        );
    }
    pub fn submit_pi(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.busy(cx) {
            return Ok(());
        }
        let (version, draft) = self
            .pi_form
            .update(cx, |form, cx| form.prepare(cx))
            .map_err(|_| "error-config-validation".to_owned())?
            .into_parts();
        let baseline = self.preferences(cx);
        let mut value = baseline.clone();
        value.pi_command = draft.normalized()?.command;
        self.start(
            ConfigRepair::SavePi,
            Some(PendingConfig {
                value,
                baseline: Some(baseline),
                version: Some(version),
            }),
            cx,
        );
        Ok(())
    }
    pub fn submit_draft(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        if self.busy(cx) {
            return Ok(());
        }
        let (version, value) = self
            .form
            .update(cx, |form, cx| form.prepare(cx))
            .map_err(|_| "error-config-validation".to_owned())?
            .map_err(|_| "error-config-validation".to_owned())?
            .into_parts();
        let pending = PendingConfig {
            value: value.normalized()?,
            baseline: Some(self.store.read(cx, |op| {
                op.data()
                    .and_then(ConfigData::configured)
                    .cloned()
                    .unwrap_or_default()
            })),
            version: Some(version),
        };
        self.start(ConfigRepair::SaveDraft, Some(pending), cx);
        Ok(())
    }
    pub fn write_committed(&mut self, cx: &mut Context<Self>) {
        if self.busy(cx) {
            return;
        }
        let pending = self.store.read(cx, |op| {
            let d = op.data()?;
            let value = d.configured()?.clone();
            Some(PendingConfig {
                value,
                baseline: None,
                version: None,
            })
        });
        if let Some(pending) = pending {
            self.start(ConfigRepair::WriteCommitted, Some(pending), cx);
        }
    }
    pub fn repair(&mut self, action: ConfigRepair, cx: &mut Context<Self>) -> Result<(), String> {
        if self.busy(cx) {
            return Ok(());
        }
        match action {
            ConfigRepair::Reload => self.reload(cx),
            ConfigRepair::SaveDraft => return self.submit_draft(cx),
            ConfigRepair::SavePi => return self.submit_pi(cx),
            ConfigRepair::SavePreference(change) => self.set_preference(change, cx),
            ConfigRepair::WriteCommitted => self.write_committed(cx),
            ConfigRepair::BackupAndReset => {
                let pending = PendingConfig {
                    value: AppConfig::default(),
                    baseline: None,
                    version: None,
                };
                self.start(action, Some(pending), cx);
            }
        }
        Ok(())
    }
    fn start(
        &mut self,
        action: ConfigRepair,
        pending: Option<PendingConfig>,
        cx: &mut Context<Self>,
    ) {
        if self.draining || self.is_running(cx) {
            return;
        }
        if matches!(action, ConfigRepair::BackupAndReset)
            && !self.store.read(cx, |op| op.problem().is_some())
        {
            return;
        }
        let version = pending.as_ref().and_then(|p| p.version);
        let rebase_setup = matches!(
            action,
            ConfigRepair::Reload | ConfigRepair::SaveDraft | ConfigRepair::BackupAndReset
        );
        let rebase_pi = rebase_setup || matches!(action, ConfigRepair::SavePi);
        let pi_version = matches!(action, ConfigRepair::SavePi)
            .then_some(version)
            .flatten();
        let reset = matches!(action, ConfigRepair::BackupAndReset);
        let reset_keybindings = matches!(
            action,
            ConfigRepair::SavePreference(PreferenceChange::ResetKeybindings)
        );
        let backup = self
            .store
            .read(cx, |op| op.problem().and_then(|p| p.backup().cloned()));
        let require_configured = self.store.read(cx, |op| {
            op.data().and_then(ConfigData::configured).is_some()
        });
        let path = self
            .store
            .read(cx, |op| op.data().map(|d| d.path.clone()))
            .map(Ok)
            .unwrap_or_else(|| self.path.clone());
        let registration = pending
            .as_ref()
            .map(|p| crate::host::prepare_shortcuts(&p.value, cx))
            .transpose();
        let started = std::time::Instant::now();
        let task = cx.spawn(async move |owner, cx| {
            let result = smol::unblock(move || {
                registration.map_err(ConfigProblem::Validation)?;
                let path = path.map_err(ConfigProblem::Read)?;
                match pending {
                    Some(pending) => write_config(path, pending, reset, reset_keybindings, backup),
                    None => read_config(path, require_configured),
                }
            })
            .await;
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis(),
                success = result.is_ok(),
                problem = result.as_ref().err().map(ConfigProblem::key),
                "configuration operation completed"
            );
            let _ = owner.update(cx, |owner, cx| {
                let value = result
                    .as_ref()
                    .ok()
                    .map(|d| d.configured().cloned().unwrap_or_default());
                let applied = value.clone().unwrap_or_else(|| owner.preferences(cx));
                crate::host::apply_shortcuts(&applied, cx);
                owner.store.update(cx, |op| op.transition(Complete(result)));
                if let Some(value) = value {
                    if rebase_pi {
                        owner.pi_form.update(cx, |form, cx| {
                            let draft = PiSettings {
                                command: value.pi_command.clone(),
                            };
                            if let Some(version) = pi_version {
                                form.rebase_if_current(version, draft, cx);
                            } else {
                                form.rebase(draft, cx);
                            }
                        });
                    }
                    if rebase_setup {
                        let _ = owner.form.update(cx, |form, cx| {
                            if let Some(version) = version {
                                form.rebase_if_current(version, value, cx);
                            } else {
                                form.rebase(value, cx);
                            }
                        });
                    }
                }
                cx.notify();
            });
        });
        self.store.update(cx, |op| match op {
            ConfigOperation::Idle(_) => op.transition(Load(task)),
            ConfigOperation::Ready(_) => op.transition(Refresh(task)),
            _ => op.transition(Repair {
                repair: action,
                task,
            }),
        });
        cx.notify();
    }
}

#[cfg(test)]
mod update_preference_tests {
    use super::AppLanguage;
    use super::PendingConfig;
    use super::PreferenceChange;
    use super::read_config;
    use super::write_config;

    #[test]
    fn update_preference_preserves_external_edits_and_reads_older_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "language = 'english'\n").unwrap();
        let baseline = read_config(path.clone(), false)
            .unwrap()
            .configured()
            .unwrap()
            .clone();
        assert_eq!(
            baseline.checks_updates_automatically(),
            cfg!(feature = "bundled")
        );
        std::fs::write(&path, "language = 'chinese'\n").unwrap();
        let mut value = baseline.clone();
        PreferenceChange::AutoCheckUpdates(false).apply(&mut value);
        write_config(
            path.clone(),
            PendingConfig {
                value,
                baseline: Some(baseline),
                version: None,
            },
            false,
            false,
            None,
        )
        .unwrap();
        let saved = read_config(path, false).unwrap();
        let value = saved.configured().unwrap();
        assert_eq!(value.language, AppLanguage::Chinese);
        assert_eq!(value.auto_check_updates, Some(false));
        assert!(!value.checks_updates_automatically());
    }

    #[gpui_kit::test]
    async fn shortcut_prepare_failure_preserves_disk_and_allows_retry(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use super::*;
        #[derive(Default)]
        struct Applied(Vec<ThemeMode>);
        impl Global for Applied {}
        cx.executor().allow_parking();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "language = 'english'\n").unwrap();
        let (form, config) = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(crate::host::Host::headless());
            cx.set_global(Applied::default());
            cx.set_global(crate::host::Host {
                prepare_shortcuts: |_, _| Err("shortcut registration failed".into()),
                apply_shortcuts: |value, cx| cx.global_mut::<Applied>().0.push(value.theme),
                refresh_menus: |_| {},
                ..crate::host::Host::headless()
            });
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::at_path(&form, Ok(path.clone()), cx));
            config.read(cx).store.clone().update(cx, |op| {
                op.transition(Load(Task::ready(())));
                op.transition(Complete(Ok(read_config(path.clone(), false).unwrap())));
            });
            (form, config)
        });
        config.update(cx, |owner, cx| {
            owner.set_preference(PreferenceChange::Theme(ThemeMode::Dark), cx)
        });
        cx.condition(&config, |owner, cx| !owner.is_running(cx))
            .await;
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "language = 'english'\n"
        );
        config.read_with(cx, |owner, cx| {
            assert!(owner.store.read(cx, |op| matches!(
                op.problem(),
                Some(ConfigProblem::Validation(_))
            )));
            assert_eq!(cx.global::<Applied>().0, [ThemeMode::System]);
        });
        cx.update(|cx| cx.global_mut::<crate::host::Host>().prepare_shortcuts = |_, _| Ok(()));
        config.update(cx, |owner, cx| {
            owner.set_preference(PreferenceChange::Theme(ThemeMode::Dark), cx)
        });
        cx.condition(&config, |owner, cx| !owner.is_running(cx))
            .await;
        assert_eq!(
            read_config(path, false)
                .unwrap()
                .configured()
                .unwrap()
                .theme,
            ThemeMode::Dark
        );
        cx.update(|cx| {
            assert_eq!(
                cx.global::<Applied>().0,
                [ThemeMode::System, ThemeMode::Dark]
            )
        });
        drop(form);
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[gpui_kit::test]
    async fn native_update_excludes_settings_and_persists_skip_before_unlocking(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use super::AppConfig;
        use super::ConfigController;
        use super::ThemeMode;
        use gpui_form::Form;
        use gpui_kit::AppContext;
        use gpui_kit::Task;
        use gpui_operation::Complete;
        use gpui_operation::Load;
        use gpui_operation::Transition;
        use gupi_updates::releases::Release;
        use gupi_updates::updates;
        use gupi_updates::updates::Status;

        cx.executor().allow_parking();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "language = 'english'\n").unwrap();
        let (_form, config, updates) = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(crate::host::Host::headless());
            updates::get(cx);
            let mut host = crate::host::Host::headless();
            host.writes_blocked = |cx| updates::current(cx).is_installing();
            host.skipped_version = |cx| match updates::current(cx).status() {
                Status::Installing(release) => Some(release.version.to_string()),
                _ => None,
            };
            host.observe_admission = |owner, cx| {
                let owner = owner.downgrade();
                let updates = updates::get(cx);
                cx.observe(&updates, move |_, cx| {
                    let _ = owner.update(cx, |_, cx| cx.notify());
                })
            };
            cx.set_global(host);
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let config = cx.new(|cx| ConfigController::at_path(&form, Ok(path.clone()), cx));
            let store = config.read(cx).store.clone();
            store.update(cx, |op| {
                op.transition(Load(Task::ready(())));
                op.transition(Complete(Ok(read_config(path.clone(), false).unwrap())));
            });
            let updates = updates::get(cx);
            updates.update(cx, |owner, cx| {
                owner.set_status_for_test(Status::Available(Release::new(
                    semver::Version::new(2, 0, 0),
                    "https://github.com/suxiaoshao/gupi/releases/tag/v2.0.0".into(),
                )));
                owner.start_install(cx).unwrap();
            });
            (form, config, updates)
        });
        config.update(cx, |owner, cx| {
            assert!(owner.busy(cx));
            owner.set_preference(PreferenceChange::Theme(ThemeMode::Dark), cx);
            owner.reload(cx);
            assert!(
                !owner.is_running(cx),
                "ordinary settings and reload must be blocked"
            );
            owner.skip_update(cx);
            assert!(
                owner.is_running(cx),
                "skip must start saving during the native session"
            );
        });
        updates.update(cx, |owner, cx| owner.finish_install(false, cx));
        config.update(cx, |owner, cx| {
            assert!(
                owner.busy(cx),
                "closing the window must not unlock an in-flight save"
            );
            owner.set_preference(PreferenceChange::Theme(ThemeMode::Dark), cx);
        });
        cx.condition(&config, |owner, cx| !owner.busy(cx)).await;
        let saved = read_config(path.clone(), false).unwrap();
        assert_eq!(
            saved.configured().unwrap().skipped_update.as_deref(),
            Some("2.0.0")
        );
        assert_eq!(saved.configured().unwrap().theme, ThemeMode::System);

        // Closing or failing another update restores the same settings entry point.
        for failed in [false, true] {
            updates.update(cx, |owner, cx| {
                owner.start_install(cx).unwrap();
            });
            assert!(config.read_with(cx, |owner, cx| owner.busy(cx)));
            updates.update(cx, |owner, cx| owner.finish_install(failed, cx));
            assert!(!config.read_with(cx, |owner, cx| owner.busy(cx)));
        }
        config.update(cx, |owner, cx| {
            owner.set_preference(PreferenceChange::Theme(ThemeMode::Dark), cx)
        });
        cx.condition(&config, |owner, cx| !owner.busy(cx)).await;
        let saved = read_config(path, false).unwrap();
        assert_eq!(saved.configured().unwrap().theme, ThemeMode::Dark);
        assert_eq!(
            saved.configured().unwrap().skipped_update.as_deref(),
            Some("2.0.0")
        );
    }
}
