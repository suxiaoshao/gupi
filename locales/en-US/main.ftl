app-title = Gupi
menu-settings = Settings
menu-show-main = Main window
menu-quit = Quit Gupi
startup-welcome = Welcome to Gupi
temporary-setup-required = Complete setup in Settings to start a temporary conversation.
startup-checking = Checking…
startup-quitting = Finishing operations…
settings-pi-command = Pi executable
settings-path-help = Leave empty to find pi automatically, or enter an absolute executable path without arguments.
settings-theme = Appearance
settings-language = Language
settings-save-pi = Save Pi path
settings-reload = Reload
settings-write-current = Write applied settings to disk (keep draft)
theme-system = System
theme-light = Light
theme-dark = Dark
language-system = System
language-english = English
language-chinese = 简体中文
recovery-config-title = Configuration needs attention
recovery-startup-title = Gupi could not start
error-instance-startup = Check the configuration folder permissions, or close other running copies of Gupi, then retry.
recovery-pi-title = Set up Pi
recovery-confirm = Continue? Reload discards your draft only after success. Reset first backs up the current file.
recovery-backup = Backup
error-config-read = Could not read the configuration. Check its location and permissions, then reload.
error-config-parse = Configuration is not valid. Reload a corrected file, or back it up and reset.
error-config-validation = Enter one executable name or an absolute executable path, without arguments.
error-config-write = Saving failed. Your draft and applied settings are retained. Check the configuration file and try again.
error-pi-probe = Pi could not be verified. Check the executable path and its Node environment, then retry.
action-check-pi = Check Pi again
action-reset = Back up and reset
action-confirm = Confirm
action-cancel = Cancel
action-save = Save
home-ready = Pi is available
home-description = This version provides environment setup and desktop preferences. Conversation support is planned for the next stage.
action-locate = Show configuration folder
error-pi-timeout = Pi did not finish the version check within 15 seconds.
error-pi-output = Pi returned more output than a version check permits.
error-pi-version = Pi exited unsuccessfully or did not return a valid version.
error-log = File logging is unavailable. Check the log directory and permissions.

setup-tagline = Your native desktop home for Pi
setup-intro = A quick setup to get started.
setup-start = Get started
setup-language-title = Feel at home
setup-search-language = Search languages…
setup-system-language = System language: { $language }
setup-appearance-title = Make it yours
setup-color-mode = Color mode
setup-system-accent = System accent
setup-pi-title = Connect Pi
setup-pi-step = Pi setup
setup-pi-help = Check a local Pi installation now, or skip this page and set it up later.
setup-check-pi = Check Pi
setup-back = Back
setup-next = Continue
setup-finish = Finish setup
setup-saving = Saving…
light-themes = Light themes
dark-themes = Dark themes

# Conversation workspace
conversation-new = New conversation
conversation-sidebar = Conversation sidebar
conversation-history = Conversation history
conversation-restoring = Restoring drafts…
conversation-discovering = Finding conversations… { $count } files
conversation-read-progress = Read { $completed } / { $total } files
conversation-refresh-progress = Refreshing { $completed } / { $total } files
conversation-project = Choose working directory
conversation-project-search-placeholder = Search projects
conversation-project-search-empty = No matching projects
conversation-open-folder = Open folder…
conversation-untitled = Untitled conversation
conversation-idle = Idle
conversation-loading = Opening session
conversation-running = Running
conversation-failed = Error needs attention
conversation-waiting = Waiting for input
conversation-search = Search conversations
conversation-refresh = Refresh sessions
conversation-refresh-current = Refresh current session
conversation-actions = Session actions
conversation-show-sidebar = Show session sidebar
conversation-hide-sidebar = Hide session sidebar
conversation-scan-failed = Could not load the conversation catalog
conversation-search-placeholder = Search names, messages, or project paths
conversation-search-empty = No matching conversations
conversation-rename = Rename…
conversation-delete = Move to Trash
conversation-delete-failed = Could not delete the conversation. See details before trying again.
conversation-copy-path = Copy path
conversation-clone = Duplicate conversation
conversation-export = Export conversation as HTML…
conversation-exported = Conversation exported to { $path }
conversation-stop = Stop generation
conversation-close-run = Close runtime
conversation-show-less = Show less
conversation-show-more = Show more…
conversation-continue = Continue from here
conversation-fork = Fork conversation from here
conversation-preview = Previewing another branch
conversation-return-current = Return to current branch
conversation-welcome = Start a conversation
conversation-welcome-project = What would you like to do in { $project }?
conversation-empty-history = No messages in this conversation yet.
conversation-bottom = Jump to bottom
conversation-copy = Copy
conversation-compaction = Context compaction summary
conversation-branch-summary = Branch summary
conversation-working = Working…
conversation-process = View process
conversation-details = Toggle details
tool-detail-offset = Starting line:
tool-detail-line-limit = Requested line limit:
tool-detail-timeout = Timeout (seconds):
tool-detail-output = Output
tool-detail-input = Input
tool-detail-additional = Additional information
tool-detail-error = Error
tool-detail-glob = File pattern:
tool-detail-ignore-case = Ignore case:
tool-detail-literal = Literal search:
tool-detail-context = Context lines:
tool-detail-result-limit = Requested result limit:
tool-detail-old-text = Requested original text
tool-detail-new-text = Requested replacement text
tool-detail-truncated = Pi truncated this output; the content shown is not the full result.
tool-detail-lines-truncated = Pi shortened some result lines.
tool-detail-match-limit = Match limit reached:
tool-detail-results-limited = Result limit reached:
tool-detail-entries-limited = Directory entry limit reached:
tool-detail-full-output = Full output file:
tool-detail-image-unavailable = Unable to display this image.
conversation-role-user = User message
conversation-role-assistant = Assistant message
conversation-role-tool = Tool result
conversation-event = Session event
conversation-close-history = Close conversation history
conversation-source = Reveal source session file
conversation-reconnect = Reconnect
conversation-interrupted = Run stopped; existing content is preserved.
conversation-no = No
conversation-submit = Submit
conversation-input = Write a message
conversation-model = Model
conversation-load-options = Connect to Pi to load models
conversation-thinking = Thinking level
conversation-unknown = Unavailable
conversation-context = Current execution branch context
conversation-auto-compaction = Auto compaction
conversation-on = On
conversation-off = Off
conversation-tokens = Total input / output
conversation-cache = Total cache read / write
conversation-cache-hit = Latest cache hit rate
conversation-cost = Cost reported by Pi
conversation-statistics = Usage statistics
conversation-send = Send (Enter); Alt+Enter queues a follow-up
conversation-sending = Sending…
conversation-graph-current = Current
conversation-graph-preview = Preview
conversation-graph-left = Show lanes to the left (or scroll horizontally)
conversation-graph-right = Show lanes to the right (or scroll horizontally)
conversation-graph-reveal = Reveal selected
conversation-graph-column = Graph
conversation-graph-message = Message
conversation-catalog-empty = No conversations yet

conversation-thinking-content = Thinking
conversation-thinking-running = Thinking…
conversation-tool-group = Tool calls ({ $count })
conversation-tool-group-read = Read files ({ $count })
conversation-tool-group-bash = Commands ({ $count })
conversation-tool-group-search = Searches ({ $count })
conversation-tool-group-edit = File changes ({ $count })
conversation-tool-line = { $action } { $summary }
conversation-tool-action-read =
    { $state ->
        [running] Reading
        [complete] Read
        [failed] Failed to read
        *[unfinished] Unfinished read
    }
conversation-tool-action-write =
    { $state ->
        [running] Writing
        [complete] Wrote
        [failed] Failed to write
        *[unfinished] Unfinished write
    }
conversation-tool-action-edit =
    { $state ->
        [running] Editing
        [complete] Edited
        [failed] Failed to edit
        *[unfinished] Unfinished edit
    }
conversation-tool-action-bash =
    { $state ->
        [running] Running
        [complete] Ran
        [failed] Command failed
        *[unfinished] Unfinished command
    }
conversation-tool-action-search =
    { $state ->
        [running] Searching
        [complete] Searched
        [failed] Search failed
        *[unfinished] Unfinished search
    }
conversation-tool-action-other =
    { $state ->
        [running] Calling { $name }
        [complete] Called { $name }
        [failed] Failed call to { $name }
        *[unfinished] Unfinished call to { $name }
    }

conversation-processed = Worked for { $duration }
conversation-processed-failed = Failed after { $duration }
conversation-processed-stopped = Stopped after { $duration }
conversation-copied = Copied
conversation-copy-failed = Copy failed. Please try again.
conversation-usage-title = Request usage
conversation-usage-model = Model
conversation-usage-provider = Provider
conversation-usage-input = Input tokens
conversation-usage-output = Output tokens
conversation-usage-cache-read = Cache read tokens
conversation-usage-cache-write = Cache write tokens
conversation-usage-total = Total tokens
conversation-usage-cost = Cost

composer-context-used = Used tokens
composer-context-limit = Context capacity
composer-context-percent = Context used
composer-token-input = Total input tokens
composer-token-output = Total output tokens
composer-token-cache-read = Total cache read tokens
composer-token-cache-write = Total cache write tokens
composer-token-usage = Session token usage

conversation-model-search = Search models…
conversation-model-empty = No matching models
conversation-thinking-off = Off
conversation-thinking-minimal = Minimal
conversation-thinking-low = Low
conversation-thinking-medium = Medium
conversation-thinking-high = High
conversation-thinking-xhigh = Extra High
conversation-thinking-max = Max

conversation-model-reasoning = Reasoning
conversation-model-vision = Vision

conversation-history-reply = Assistant reply
conversation-history-brief = Brief: user messages and final answers
conversation-history-detailed = Detailed: messages, tools and summaries
conversation-history-progress = Assistant progress
conversation-history-thinking = Thinking
conversation-history-calls = Tool calls
conversation-history-failed = Execution failed
conversation-history-stopped = Interrupted
conversation-history-empty = Reply without text
composer-model-thinking = Model and thinking level
composer-model-refresh = Reload models and thinking levels
composer-thinking-unavailable = Not supported
composer-model-loading = Loading models…
composer-thinking-loading = Loading thinking levels…
composer-model-confirming = Confirming model settings…
composer-stats-loading = Loading usage…
composer-stats-retry = Retry loading usage
conversation-checking-file = Checking conversation file…
conversation-connecting = Connecting to Pi…
conversation-history-loading = Loading conversation history…
conversation-history-refreshing = Refreshing conversation history…
conversation-fork-options-loading = Loading fork options…
conversation-fork-options-retry = Retry loading fork options
history-canvas-zoom-in = Zoom in
history-canvas-zoom-out = Zoom out
history-canvas-fit = Fit tree (0)
history-canvas-current = Locate execution node
history-canvas-expand = Expand { $count } nodes
history-canvas-collapse = Collapse { $count } process nodes, keeping navigation markers
history-canvas-empty = No history nodes to display
history-canvas-help = Conversation tree: scroll or drag to pan, pinch or plus/minus to zoom, arrow keys to select, Enter to preview, E to expand the next segment

history-view-tree = Tree
history-view-list = List
history-level-brief = Brief
history-level-detailed = Detailed
history-level-all = All
history-level-all-description = All: including settings, labels and custom records
history-scope-all = All branches
history-scope-branch = This branch only
history-model-change = Model change
history-thinking-change = Thinking level change
history-session-info = Session information change
history-label-change = Label change
history-custom-record = Custom record


history-content = Content
history-range = Scope

command-palette = Command Palette
command-focus-input = Focus Conversation Input
command-model = Select Model and Thinking Level…
command-copy-last-answer = Copy Last Answer
command-compact = Compact Context
conversation-compacting = Compacting context…
command-history-description = View history or choose a user message to fork
command-show-history = Show History
command-hide-history = Hide History
command-current-session = Current session:
command-unavailable = Unavailable in the current state
command-target-changed = This conversation is no longer available. Select another conversation.
command-scanning = Loading conversations…
command-loading = Loading Pi commands…
command-no-matches = No matching Pi commands
conversation-reconnect-unconfirmed = Pi process exit could not be confirmed. Restart Gupi before reconnecting.
action-retry = Retry

command-search-placeholder = Search actions or enter a command
command-empty = No Pi commands available in this session

command-group-app = Application
command-group-extensions = Extension commands
command-group-skills = Skills
command-group-prompts = Prompt templates
command-scope-user = Personal
command-scope-project = Project
command-scope-temporary = Temporary
command-connection-unavailable = Pi is not ready
command-dismiss = Close
command-complete = Complete
command-send-text = Send
command-execute = Run

command-scope-current = Current session

conversation-working-duration = Working { $duration }
conversation-tool-group-skill = Skills read ({ $count })
conversation-shell-line = { $shell } · { $action } { $summary }
conversation-tool-action-skill =
    { $state ->
        [running] Reading skill
        [complete] Read skill
        [failed] Failed to read skill
       *[unfinished] Skill read unfinished
    }

# Unified settings
settings-page-general = General
settings-page-pi = Pi
settings-page-keys = Keyboard shortcuts
settings-page-skills = Skills
settings-page-prompts = Prompts
settings-page-about = About
settings-key-reset = Restore default
settings-key-reset-all = Restore all defaults
settings-key-reset-all-confirm = Restore the default bindings for all shortcuts? All custom shortcuts will be cleared.
settings-key-clear = Clear shortcut
settings-key-cancel = Cancel changes
settings-key-unbound = No shortcut assigned
settings-key-recording = Press a shortcut, or Esc to cancel
settings-key-conflict = Conflicting or invalid shortcut
settings-key-invalid = Enter a valid shortcut, or leave empty to unbind
settings-resource-refresh = Reload
settings-resource-reload-help = Personal resources only. Reload existing sessions manually to apply changes.
settings-resource-working = Working…
settings-resource-pi-required = Save and check a working Pi executable in Pi settings first.
settings-resource-invalid-name = Use a valid filename without path separators.
settings-resource-register = Add local
settings-resource-create = Create
settings-resource-empty = No matching resources.
settings-resource-delete = Move to trash
settings-resource-save = Save text
shortcut-save-task = Save
settings-resource-loading = Reading text…
settings-resource-confirm-remove = Only the selected file moves to Trash; its directory is kept.
settings-package-install = Install package
settings-package-update = Update
settings-package-remove = Remove
settings-package-source-help = Package source: npm:name, a Git URL, or an absolute local path.
settings-package-extensions = Extensions { $count }
settings-package-skills = Skills { $count }
settings-package-prompts = Templates { $count }
settings-package-themes = Themes { $count }
settings-skill-search = Search skills by name, description, path or source
settings-skill-collapse = Collapse
settings-resource-edit = Edit…
settings-about-gupi = Version
settings-about-pi = Version
settings-about-path = Resolved path
settings-about-status = Probe status
settings-about-unavailable = Not available yet
settings-editor-discard = Discard unsaved text changes?
settings-package-confirm-remove = Removes the package from the selected Pi scope. Local source folders are kept.
settings-package-heading = Installed packages
settings-extension-heading = Standalone extensions
settings-template-heading = Command templates
settings-system-heading = System prompts
settings-config-heading = Configuration file
settings-config-open = Open configuration file
settings-mode-help = Follow the system to switch between your light and dark themes automatically.
settings-light-help = Used when light mode is active.
settings-dark-help = Used when dark mode is active.
settings-pi-unsaved = Unsaved path. Checking validates the current input without saving it.
settings-pi-saved = The current path is saved. Check again to verify Pi is available.
settings-resource-location = Show location
settings-package-source = Package source
settings-package-empty = No personal Pi packages installed.
settings-package-resources-empty = No declared resources were found in this package.
settings-resource-name = Name
settings-resource-none = No personal resources of this type yet.
settings-resource-view = View
settings-resource-readonly-badge = Read-only
settings-source-personal = Personal
settings-source-external = External
settings-system-replace = Replace the default system prompt
settings-system-replace-help = Completely replaces Pi’s default system prompt.
settings-system-append = Append personal instructions
settings-system-append-help = Keeps the default system prompt and appends these instructions.
settings-resource-success = Completed the operation on “{ $target }”. Reload existing sessions manually.
settings-resource-failed = The operation on “{ $target }” failed.
settings-about-help = Shows the latest check of the saved path. Open Pi settings to check again.
settings-key-reveal-session = Show session file

settings-key-group-app = Application
settings-key-group-conversation = Conversation
settings-key-group-files = Session files

settings-package-kind-extensions = Extensions
settings-package-kind-skills = Skills
settings-package-kind-themes = Themes
settings-package-kind-prompts = Templates
settings-package-expand = Expand package contents
settings-package-collapse = Collapse package contents

settings-template-search = Search template names, descriptions, or sources
settings-template-collapse = Collapse preview

temporary-title = Temporary conversations
attachment-add = Attach files
attachment-remove = Remove attachment
attachment-file = File
attachment-clipboard = Clipboard image
image-preview-close = Close preview
image-preview-open = Open image preview
image-preview-zoom-in = Zoom in
image-preview-zoom-out = Zoom out
shortcut-global = Global shortcuts
shortcut-launcher = Temporary window
shortcut-add = Add template task…
shortcut-edit = Edit template task…
shortcut-delete = Delete template task
shortcut-name = Name
shortcut-template = Command template
shortcut-source = Input source
shortcut-selection = Selected text
shortcut-clipboard = Clipboard
shortcut-fallback = Selected text, otherwise clipboard
shortcut-model = Model
shortcut-thinking = Thinking level
shortcut-default-model = Pi default model
shortcut-default-thinking = Pi default thinking level
shortcut-personal = Personal
shortcut-template-required = Select a command template.
shortcut-reload-options = Reload options
temporary-clean-released = Released workspaces
temporary-clean = Clean up
temporary-clean-help = Move released temporary workspaces to Trash. Active conversations are kept.
temporary-cleaned = Workspaces moved to Trash
shortcut-tasks = Template tasks

shortcut-select-template = Choose a prompt template

temporary-search-placeholder = Search temporary conversations
temporary-search-empty = No matching temporary conversations
temporary-toggle-input = Switch input
temporary-hide = Hide
session-search-open = Open conversation

temporary-actions = Actions
temporary-paste-answer = Paste Last Answer
temporary-reveal-workspace = Show Working Directory
temporary-search-actions = Search for actions…
temporary-switch-session = Switch Temporary Conversation
temporary-paste-failed = Answer copied. Automatic paste failed. Check Accessibility permission and the target application, or paste manually.
temporary-session-1 = Switch to Temporary Conversation 1
temporary-session-2 = Switch to Temporary Conversation 2
temporary-session-3 = Switch to Temporary Conversation 3
temporary-session-4 = Switch to Temporary Conversation 4
temporary-session-5 = Switch to Temporary Conversation 5
temporary-session-6 = Switch to Temporary Conversation 6
temporary-session-7 = Switch to Temporary Conversation 7
temporary-session-8 = Switch to Temporary Conversation 8
temporary-session-9 = Switch to Last Temporary Conversation
temporary-send = Send
temporary-trash = Move Temporary Conversation to Trash
settings-key-stop-or-hide = Stop Generation / Hide Temporary Window

conversation-queue-title = Queued ({ $count })
conversation-queue-steer = Steer current turn
conversation-queue-follow-up = Follow-up task
conversation-queue-restore = Return all queued text to draft
conversation-queue-clear = Clear all queued messages
conversation-queue-no-text = Message without text
conversation-queue-unavailable = Queue content is not available yet
conversation-queue-text-only = Returning the queue restores text only; queued images cannot be restored.

conversation-retry-countdown = Retry { $attempt }/{ $total } in about { $seconds }s
conversation-retry-waiting = Retry { $attempt }/{ $total }: waiting for Pi
conversation-summary-retry-countdown = Summary retry { $attempt }/{ $total } in about { $seconds }s
conversation-summary-retry-waiting = Summary retry { $attempt }/{ $total }: waiting for Pi

message-details-open = View details…
message-details-copy-all = Copy all
tool-detail-command = Command
tool-detail-content = Content
tool-detail-changes = Changes

settings-notifications = Notifications
settings-notification-help = System notifications use a generic description. Open a notification to return to its conversation. Unread counts clear when that conversation is open in the foreground.
settings-notification-waiting = Notify when input is needed in the background
settings-notification-failures = Notify when a task fails in the background
settings-notification-plugins = Send plugin reminders as background system notifications
settings-notification-attention = Request attention when background input is needed
settings-notification-completion = Answer completion notifications
notification-off = Off
notification-background = Background only
notification-always = Always
notification-waiting = Waiting for your input
notification-completed = Your answer is ready
notification-failed = A task needs attention after a failure
notification-plugin = Plugin reminders
notification-unread = Unread conversations
notification-running = Running
notification-clear = Clear reminders
notification-session-notices = Conversation reminders

settings-icon-theme = Application icon
settings-icon-theme-help = Changes the in-app logo and the running app’s macOS Dock icon. Finder and the Windows taskbar keep the packaged icon.
icon-theme-classic = Classic
icon-theme-classic-gradient = Classic gradient
icon-theme-color = Pi color
icon-theme-color-gradient = Pi gradient
icon-theme-pride = Rainbow
icon-theme-ukraine = Ukraine
icon-theme-ukraine-gradient = Ukraine gradient

language-traditional-chinese = 繁體中文
language-japanese = 日本語
language-korean = 한국어
language-german = Deutsch
language-french = Français
language-spanish = Español
language-portuguese-brazil = Português (Brasil)
menu-about = About Gupi
menu-conversation = Conversation
menu-edit = Edit
menu-view = View
menu-window = Window
menu-help = Help
menu-services = Services
menu-hide = Hide Gupi
menu-hide-others = Hide Others
menu-show-all = Show All
menu-undo = Undo
menu-redo = Redo
menu-cut = Cut
menu-copy = Copy
menu-paste = Paste
menu-select-all = Select All
menu-minimize = Minimize
menu-zoom = Zoom
menu-fullscreen = Toggle Full Screen
menu-docs = Gupi User Guide
menu-pi-docs = Pi Documentation
menu-report-issue = Report an Issue
menu-logs = Show Logs
menu-copy-diagnostics = Copy Diagnostics
settings-permissions = System permissions
settings-accessibility-help = Selected text and pasting into another application may require Accessibility access. Enable Gupi in System Settings, then try the action again. The answer remains available to copy if pasting fails.
settings-accessibility-open = Accessibility Settings
settings-notification-permission-help = If alerts or the Dock badge are missing, allow Gupi notifications and badges in System Settings. Permission is requested when the feature is first used.
settings-notification-permission-open = Notification Settings
settings-native-language-help = The app language changes immediately. Restart Gupi to apply it to macOS system dialogs. Windows system dialogs follow the Windows display language.
settings-diagnostics-help = Includes the app version, platform, Pi command and configuration and log locations. Does not include conversations, credentials or environment variables.

setup-preferences-title = Language and appearance
setup-preferences-step = Language and appearance
setup-desktop-title = Shortcuts and notifications
setup-desktop-step = Shortcuts and notifications
setup-desktop-help = You can change these choices later in Settings.
setup-page-skip = Skip this page
setup-skip-all = Use defaults
setup-pi-later = Set up Pi later
action-open = Open

setup-appearance-options = Choose a theme and icon
setup-pi-install-guide = Install Pi
setup-pi-model-guide = Connect a model

conversation-plugin-message = Plugin message
conversation-session-info-command = Session information…
conversation-session-info-title = Session information
conversation-session-info-identity = Identity and location
conversation-session-info-name = Conversation
conversation-session-info-id = Pi session ID
conversation-session-info-directory = Working directory
conversation-session-info-file = Session file
conversation-session-info-not-saved = Not saved yet
conversation-session-info-runtime = Runtime configuration
conversation-session-info-model = Model
conversation-session-info-thinking = Thinking level
conversation-session-info-usage = Usage
conversation-session-info-input = Input tokens
conversation-session-info-output = Output tokens
conversation-session-info-cache-read = Cache read
conversation-session-info-cache-write = Cache write
conversation-session-info-cost = Cost reported by Pi
conversation-session-info-context = Current context (tokens / window / percentage)
conversation-session-info-context-value = { $tokens } of { $window } tokens ({ $percent })
conversation-session-info-counts = Session counts
conversation-session-info-user-messages = User messages
conversation-session-info-assistant-messages = Assistant messages
conversation-session-info-tool-calls = Tool calls
conversation-session-info-tool-results = Tool results
conversation-session-info-total-messages = Total messages

conversation-find = Find in conversation
conversation-find-placeholder = Find in conversation
conversation-find-count = { $current } of { $total }
conversation-find-previous = Previous match (Shift+Enter)
conversation-find-next = Next match (Enter)
conversation-find-close = Close find (Esc)

# Composer resources
command-select-resource = Insert into message
command-group-files = Files and folders
resource-files-loading = Searching project files and folders…
resource-open-file = Open file
resource-skill-content = Skill instructions
resource-template-unreadable = Cannot read the prompt template. Reload resources and try again.
resource-template-unused-files = This template does not include all referenced file arguments. Add the missing parameters (for example, $ARGUMENTS), or remove those references before sending.
resource-search-files = Search project files and folders…
resource-choose-files = Choose files or folders…

resource-edit-in-composer = Choose a resource here; edit its arguments in the message input.
resource-template-unclosed-quote = Close the quote in the template arguments before sending file references.

resource-answer-required = Choose an answer before submitting.

menu-check-updates = Check for updates…
updates-title = Updates
updates-check = Check for updates
updates-automatic = Automatically check for updates
updates-automatic-help = Checks GitHub for new versions hourly. Downloads start only when you choose.
updates-idle = Check whether a new version of Gupi is available.
updates-checking = Checking for updates
updates-current = You’re up to date
updates-unpublished = No stable release has been published yet.
updates-available = Gupi { $version } is available
updates-release = Release notes and downloads
updates-install-help = Use the installer or package manager you used to install Gupi.
updates-error-network = Couldn’t check for updates. Check your connection and try again.
updates-error-rate-limit = GitHub’s request limit was reached. Try again later.
updates-error-response = Couldn’t read the release information. Try again later.

updates-install = Download and install…
updates-native-progress = Complete the update in the update window.
updates-install-failed = Couldn’t complete the update. Try again, or reopen Gupi and check its logs.
updates-restart-help = Installing will save drafts, close Pi connections, and restart Gupi.
updates-show-progress = Show update progress…

shortcut-screenshot = Screenshot
screenshot-failed = Couldn’t capture the screen. Try again.
screenshot-permission-required = Screen recording access is required. Enable Gupi in System Settings → Privacy & Security → Screen Recording, then try again. If macOS asks you to restart Gupi, restart it first.

# Project files and source preview
files-title = Files
files-panel = Workspace
files-close = Close panel
files-refresh = Reload
files-preview = Preview
files-readonly = Readonly
files-wrap = Wrap lines
files-reload = Reload
files-loading = Reading…
files-expanded = Expanded
files-collapsed = Collapsed
files-incomplete = Some entries could not be read. Retry
files-read-failed = Could not read this file or directory. Retry
files-unsupported = This file does not support text preview
files-too-large = File exceeds the preview limit (1 MiB or 20,000 lines)
files-long-line = Long lines: syntax highlighting is disabled
files-no-session = Select a conversation to browse project files

files-actions = File actions
files-copy-path = Copy path
files-reveal = Reveal in files
files-close-source = Close file
files-focus-source = Focus source
files-conversation = Conversation
files-conversation-busy = Conversation, responding
files-view = View
files-source = Source

files-link-label = { $name } — symbolic link
files-link-directory-label = { $name } — linked folder, { $state }
files-link-unavailable-label = { $name } — link target unavailable
files-link-unavailable = Link target unavailable

# Settings and shortcuts redesign
settings-page-packages = Packages
settings-app-language = App language
settings-config-location = Location
settings-temporary-storage = Temporary conversation storage
settings-accessibility = Accessibility
settings-open-system-settings = Open system settings
settings-color-mode-label = Mode
settings-light-help-active = Used when light mode is active. In use now.
settings-dark-help-active = Used when dark mode is active. In use now.
settings-icon-theme-help-other = Changes the in-app logo. The taskbar and file manager keep the packaged icon.
settings-notification-background-heading = Background activity
settings-notification-completion-heading = Answer completion
settings-notification-permission-help-windows = If alerts are missing, allow Gupi notifications in Windows Settings. Permission is requested when the feature is first used.
settings-pi-status-ready = Available · { $version } · { $path }
settings-pi-draft = Unsaved
settings-pi-saved-tag = Saved
settings-diagnostics = Diagnostics
settings-open-pi = Open Pi settings
settings-resource-actions = More actions
settings-resource-delete-ellipsis = Move to trash…
settings-resource-from-package = From { $package }
settings-resource-open-package = Show package
settings-key-customized = Customized
settings-key-customized-only = Customized only
settings-key-group-system = System-wide
settings-key-system-help = These shortcuts work while Gupi is running, even when another app is active.
settings-key-group-workspace = Workspace & files
settings-key-reset-all-ellipsis = Restore all defaults…
settings-key-confirm = Confirm
settings-key-unsaved = Unsaved
settings-key-scope-any-window = Any Gupi window
settings-key-scope-palette = Main, temporary and startup windows
settings-key-scope-conversation = Conversations in the main and temporary windows
settings-key-scope-messages = When the message list has focus
settings-key-scope-stop = Main window: stop generating · Temporary window: hide
settings-key-scope-temporary = Temporary window
settings-key-scope-temporary-paste = Temporary window; plain Enter does not fire in text fields
settings-key-search = Search shortcuts
settings-key-search-keystroke = Search by keys
settings-key-search-clear = Clear key search
settings-key-sessions = Switch to temporary conversation 1–9
settings-key-sessions-expand = Show all
settings-key-sessions-collapse = Collapse
settings-key-show-conflict = Show
settings-key-used-by = Already used by “{ $command }” ({ $scope })
settings-key-task-actions = Task actions
settings-key-task-description = /{ $template } · { $source }
settings-key-task-enabled = Enable task
shortcut-delete-ellipsis = Delete template task…

# Package catalog
packages-installed-tab = Installed
packages-browse-tab = Browse
packages-search = Search packages
packages-source-npm = Results come from npm packages tagged “pi-package”.
packages-directory = Pi package directory
packages-loading = Loading…
packages-retry = Retry
packages-installed = Installed
packages-previous = Previous
packages-next = Next
packages-back = Back
packages-latest = Latest published { $version }
packages-publisher = Published by { $publisher }
packages-repository = Repository
packages-npm = npm page
packages-install-source = Install source
packages-declared = Declared contents
packages-undeclared = The package does not declare its contents; they are listed after install.
packages-installed-version = Installed { $version }
packages-files-missing = Files missing
packages-trust = Packages run code with your permissions.
packages-done = Done. Reload existing sessions or start a new one to load the change.
packages-failed = Pi reported an error:
packages-install = Install
packages-copy-source = Copy install source
packages-open-repository = Open repository
packages-open-npm = Open npm page
packages-problem-network = Couldn’t reach the npm registry. Check your connection and try again.
packages-problem-rate-limited = The npm registry is limiting requests. Try again later.
packages-problem-not-found = Package not found.
packages-problem-invalid = The npm registry returned an unexpected response.
settings-package-install-source = Install from source…
settings-package-remove-ellipsis = Remove…

# Package browser layout
packages-source-short = npm · pi-package
packages-page = Page { $page }
packages-page-empty = No matching packages on this page.
packages-close = Close details
packages-local-unavailable = Local package information is unavailable. Refresh resources to enable package actions.
packages-refresh-details = Reload

## Pi settings
settings-domain-label = Settings area
settings-domain-gupi = Gupi
settings-domain-pi = Pi
settings-page-connection = Connection
settings-page-pi-conversation = Conversation
settings-page-pi-network = Network
pi-group-model = Default model
pi-group-context = Context
pi-group-queue = Message queue
pi-group-images = Images
pi-group-proxy = Proxy
pi-group-retry = Retry
pi-field-model = Default model
pi-field-model-help = Default for new conversations; model scope may select a different initial model.
pi-field-model-auto = Automatic
pi-field-model-unavailable = This model isn't currently available.
pi-field-thinking = Default thinking level
pi-thinking-unsupported = { $level } (not supported by this model)
pi-field-compaction = Automatic compaction
pi-field-reserve = Reserved response tokens
pi-field-reserve-help = Tokens kept free for the model's reply when compacting.
pi-field-keep = Recent tokens kept
pi-field-keep-help = Recent context that compaction doesn't summarize.
pi-field-steering = Steering messages
pi-field-follow-up = Follow-up messages
pi-delivery-one = One at a time
pi-delivery-all = All at once
pi-field-auto-resize = Resize large images
pi-field-auto-resize-help = Scales images to at most 2000×2000 before sending.
pi-field-block-images = Don't send images to the model
pi-field-proxy = HTTP proxy
pi-field-proxy-help = Applies to requests made by Pi, not by Gupi.
pi-edit-in-global = Edit in Global
pi-value-unset = Not set
pi-field-retry = Automatic retry
pi-field-max-retries = Maximum retries
pi-field-base-delay = Initial delay (ms)
pi-field-max-delay = Maximum delay (ms)
pi-source-default = Default
pi-source-global = Global
pi-source-project = Project
pi-unsaved = Unsaved
pi-changed-on-disk = Changed in the file since it was read.
pi-invalid-count = Enter a whole number of 0 or more.
pi-menu-revert = Undo change
pi-menu-pin-project = Set for this project
pi-menu-pin-global = Set in global settings
pi-menu-inherit = Use global value
pi-menu-default = Use default
pi-field-actions = Field actions
pi-scope-global = Global
pi-scope-label = Settings scope
pi-trust-untrusted = Not trusted
pi-trust-untrusted-help = Pi won't use this project's settings until the project is trusted.
pi-trust-action = Trust project…
pi-trust-title = Trust “{ $name }”?
pi-trust-body = Pi will load this folder's project settings and resources, install its project packages and run its project extensions. Parent folders aren't trusted.
pi-trust-confirm = Trust project
pi-settings-save = Save
pi-settings-discard = Discard
pi-settings-file-actions = More actions
pi-settings-open-file = Open settings.json
pi-settings-reload = Reload
pi-settings-invalid = Fix the highlighted fields before saving.
pi-settings-conflict = Some settings changed in the file after they were read. Reload, then save again.
pi-settings-locked = The settings file is in use by another process. Try again.
pi-settings-unsaved-title = Save Pi settings?
pi-settings-unsaved-body = Your changes to Pi settings haven't been saved.
pi-settings-loading = Loading…
pi-settings-unsupported = Editing Pi settings here requires Pi 1.1.0 or later. Installed: { $version }.
pi-settings-unavailable = Pi isn't available. Check the Connection page.
pi-models-loading = Loading models…
pi-models-retry = Retry
pi-project-settings = Project Pi settings…

pi-scope-choose-folder = Choose project folder…
pi-scope-choose-folder-action = Choose
settings-resource-group-project = This project
settings-resource-group-inherited = From global
settings-source-project = This project
settings-resource-same-global = Same name as global · project first
settings-resource-same-project = Same name as project · project first
settings-resource-not-loaded = Not loaded (project not trusted)
settings-resource-edit-global = Edit in global
settings-resource-none-project = This project's .pi folder has none yet.
settings-system-project-file = Project file
settings-system-uses-global = Uses global
settings-system-use-global = Use global
settings-system-use-global-title = Use global { $file }?
settings-system-use-global-body = The project file moves to the Trash and Pi falls back to the global file.
settings-editor-changed = The file changed after it was opened, so it was not saved. Reload to discard these edits, or cancel.
settings-editor-reload = Reload
settings-editor-unsaved-title = Save changes to { $file }?
settings-editor-unsaved-body = Your text changes are not saved yet.

pi-project-settings-folder = Project Pi settings — { $folder }

packages-install-project = Install to project { $folder }
packages-project-delta = Global package filters · read-only
packages-project-filtered = Filtered in this project
packages-project-replaced = Replaced by project
packages-project-replaces = Replaces global package

settings-resource-disabled = Disabled
settings-theme-heading = Independent themes
settings-project-package-empty = No packages in this project.

pi-model-scope = Model scope
pi-model-scope-help = New conversations use the default model if it matches this scope, otherwise the first matching model. This does not restrict models you can select in a conversation.
pi-model-scope-add-model = Add model…
pi-model-scope-add-pattern = Add pattern
pi-model-scope-up = Move up
pi-model-scope-down = Move down
pi-model-scope-delete = Remove pattern
pi-model-scope-unrestricted = Unrestricted
pi-model-scope-unrestricted-project = Unrestricted (overrides global)
pi-model-scope-blank = Patterns cannot be blank.
pi-model-scope-malformed = This value must be an array of strings. Correct it in the settings file.

conversation-edited-files = { $count ->
    [one] { $count } file edited
   *[other] { $count } files edited
    }
conversation-edit-write = Write
conversation-edit-error = Error
conversation-written-content = Written content
conversation-edit-operation = Operation { $index }
conversation-edit-more-files = { $count ->
    [one] Show { $count } more file
   *[other] Show { $count } more files
    }
conversation-edit-less-files = Show fewer files
conversation-edit-more-operations = { $count ->
    [one] Show { $count } more operation
   *[other] Show { $count } more operations
    }
conversation-edit-previous-operations = Previous operations
conversation-edit-stats-tooltip = Net added and removed lines from combined recorded edit patches. Writes and files that could not be combined are excluded; this is not a complete workspace diff.
conversation-edit-file-label = { $path } — { $state }
conversation-edit-operation-stats-tooltip = Added and removed lines in this recorded patch.
conversation-edit-uncombined = Could not combine; showing operations

# Advanced Pi settings
settings-page-pi-tools = Tools & environment
pi-group-tools = Tools
pi-group-shell = Shell
pi-group-package-command = Package manager
pi-group-privacy-trust = Privacy & trust
pi-group-advanced = Advanced
pi-group-provider = Provider-level retries
pi-field-default-tools = Default tools
pi-field-codemode-mode = Codemode mode
pi-field-codemode-budget = Declaration budget
pi-field-shell-path = Shell path
pi-field-shell-prefix = Command prefix
pi-field-npm-command = npm command
pi-field-telemetry = Installation telemetry
pi-field-project-trust = Default project trust
pi-field-model-thinking = Per-model thinking levels
pi-field-model-overrides = Per-model compaction
pi-field-budget-minimal = Minimal
pi-field-budget-low = Low
pi-field-budget-medium = Medium
pi-field-budget-high = High
pi-field-cache-warming = Cache warming
pi-field-transport = Transport
pi-field-http-timeout = HTTP idle timeout (ms)
pi-field-websocket-timeout = WebSocket connection timeout (ms)
pi-field-provider-timeout = Request timeout (ms)
pi-field-provider-retries = Retry attempts
pi-field-provider-delay = Maximum retry delay (ms)
pi-timeout-disabled = Off
pi-tools-startup-help = Extensions and MCP may enable additional tools. This is not a security restriction or a live tool list.
pi-map-add-model = Add model
pi-map-remove-local = Remove local override
pi-model-thinking-help = Explicit levels and saved sessions take precedence at startup; switching models uses this value. A thinking suffix in the model scope takes precedence.
pi-model-overrides-help = Each token value inherits independently. Only changed leaves are saved.
pi-trust-always-title = Trust projects by default?
pi-trust-always-body = Pi will trust projects without an applicable decision, load their settings and execute project extensions. Existing explicit decisions and trust extensions still take precedence.
pi-telemetry-env-on = PI_TELEMETRY overrides this preference: enabled
pi-telemetry-env-off = PI_TELEMETRY overrides this preference: disabled
pi-codemode-help = Codemode settings require the enabled built-in codemode extension and an active codemode tool.
pi-thinking-budget-help = Only applies to providers and models that support token budgets.
pi-cache-warming-help = Warming requests incur usage.
pi-telemetry-help = Controls installation telemetry, not update checks or all analytics.
pi-provider-retry-help = Pi recommends keeping this at 0.
pi-trust-always-confirm = Trust by default
pi-choice-ask = Ask
pi-choice-always = Always trust
pi-choice-never = Never trust
pi-choice-codemode-on = Direct tools and Codemode
pi-choice-codemode-only = Codemode only
pi-choice-streaming = During streaming
pi-choice-idle = Also while idle
pi-choice-auto = Automatic

pi-thinking-budgets = Thinking budgets (tokens)
pi-model-unavailable-short = Unavailable
pi-models-unavailable = Couldn’t load models
pi-map-model = Model
pi-map-reserve = Reserve tokens
pi-map-keep = Keep recent tokens
pi-map-token-action = { $field }: { $action }
pi-tools-add = Add
pi-tools-custom = Custom…
pi-tool-remove = Remove { $tool }
pi-tools-preview = At startup: { $tools }
pi-npm-invalid = Enter a command with balanced quotes and a nonempty first argument.
pi-timeout-off = Disabled
pi-npm-help = Shell quoting preserves spaces and empty arguments.
pi-budget-input = { $level } thinking budget (tokens)

pi-group-resource-paths = Resource paths
pi-field-resource-extensions = Extensions
pi-field-resource-skills = Skills
pi-field-resource-prompts = Prompts
pi-field-resource-themes = Themes
pi-resource-paths-help = Raw entries in this scope’s file, in order.
pi-resource-unsupported = Unsupported format · read-only
pi-resource-absent = Key absent
pi-resource-empty = Empty array []
pi-resource-up = Move up
pi-resource-down = Move down
pi-resource-remove = Remove entry
pi-resource-add = Add
pi-resource-clear = Clear array []
pi-resource-remove-key = Remove key
pi-resource-entry-actions = Entry actions

# Pi configuration files
pi-files-models = Models & providers
pi-files-mcp = MCP
pi-files-startup-only = Pi reads this file when a new session starts. Existing sessions are not reloaded.
pi-files-global-only = This file is global only. Choose the global scope to edit it.
pi-files-comments = Saving removes comments from this file.
pi-files-add = Add…
pi-files-edit = Edit…
pi-files-delete = Delete
pi-files-override = Override
pi-files-definition = Full definition
pi-files-disabled = Disabled
pi-files-duplicate = This name conflicts with an existing entry.
pi-files-invalid = Could not read or save this entry. Check its fields and the configuration file.
pi-files-secret-set = Set · keep
pi-files-secret-keep = Keep
pi-files-secret-replace = Replace with new value
pi-files-secret-remove = Remove
pi-files-name = Name
pi-files-key = Key
pi-files-model = Model
pi-files-add-model = Add model
pi-files-on = On
pi-files-off = Off

# Structured configuration editor
pi-files-label-name = Display name
pi-files-label-id = Model ID
pi-files-label-baseUrl = Base URL
pi-files-label-api = API
pi-files-label-apiKey = API key
pi-files-label-authHeader = Authorization header
pi-files-label-oauth = OAuth
pi-files-label-compat = Compatibility
pi-files-label-models = Models
pi-files-label-modelOverrides = Model overrides
pi-files-label-enabled = Enabled
pi-files-label-exposure = Exposure
pi-files-label-type = Transport
pi-files-label-description = Description
pi-files-label-command = Command
pi-files-label-url = URL
pi-files-label-cwd = Working directory
pi-files-label-timeout = Timeout
pi-files-label-toolExposure = Tool exposure
pi-files-label-args = Arguments
pi-files-label-env = Environment variables
pi-files-label-headers = Headers
pi-files-label-clientId = Client ID
pi-files-label-clientSecret = Client secret
pi-files-label-callbackPort = Callback port
pi-files-label-callbackUrl = Callback URL
pi-files-label-scope = Scopes
pi-files-label-clientName = Client name
pi-files-label-clientRegistration = Client registration
pi-files-label-authServerMetadataUrl = Authorization metadata URL
pi-files-label-provider = Provider
pi-files-label-reasoning = Reasoning
pi-files-label-contextWindow = Context window
pi-files-label-maxTokens = Maximum output tokens
pi-files-label-input = Input modalities
pi-files-label-cost = Cost
pi-files-label-thinkingLevelMap = Thinking level map
pi-files-label-inputLimits = Input limits
pi-files-label-promptCache = Prompt cache
pi-files-label-samplingParams = Sampling parameters
pi-files-label-samplingParamsByThinkingLevel = Sampling by thinking level
pi-files-label-autoEnableCodemode = Automatically enable Codemode
pi-files-providers = Providers
pi-files-servers = MCP servers
pi-files-automation = Codemode
pi-files-empty = No entries
pi-files-basic = Basic
pi-files-connection = Connection
pi-files-exposure-section = Exposure
pi-files-authentication = Authentication
pi-files-advanced = Advanced
pi-files-back = Back
pi-files-stage = Stage changes
pi-files-secret-present = Set
pi-files-add-value = New value
pi-files-confirm-add = Add
pi-files-definition-kind = Definition
pi-files-provider-summary =
    { $models ->
        [one] { $models } model
       *[other] { $models } models
    } · { $overrides ->
        [one] { $overrides } override
       *[other] { $overrides } overrides
    }

# Pi provider authentication
pi-auth-title = Authentication
pi-auth-help = Credentials are stored in Pi’s auth.json. Changes take full effect in new sessions. Gupi does not reconnect or synchronize model lists in existing sessions.
pi-auth-add = Add API key…
pi-auth-replace = Replace API key…
pi-auth-api-key = API key
pi-auth-oauth = OAuth
pi-auth-unknown = Unrecognized
pi-auth-missing = No stored credentials
pi-auth-set = Already set
pi-auth-empty = No stored providers. Add an API key to get started.
pi-auth-provider = Provider ID
pi-auth-key-placeholder = Enter a new API key
pi-auth-saving = Saving…
pi-auth-candidates = Choose a provider
pi-auth-check = Check configuration
pi-auth-checking = Checking configuration
pi-auth-ready = Pi found usable credential configuration. This does not validate the token.
pi-auth-not-ready = Pi could not find usable credential configuration.
pi-auth-invalid = Pi reported invalid configuration.
pi-auth-check-failed = Could not check configuration.
pi-auth-provider-not-found = Provider not found.
pi-auth-not-configured = Credentials are not configured.
pi-auth-not-available = Credentials are not available.
pi-auth-invalid-state = Pi could not read the authentication state.
pi-auth-copy-login = Copy login command
pi-auth-login-help = Login command copied. Run it in the Pi terminal to log in again.
pi-auth-oauth-help = Local record only. Pi may try to refresh credentials when using them. Log in again if needed.
pi-auth-expired = Past the recorded expiration time
pi-auth-expires = Recorded expiration: { $time }
pi-auth-remove = Remove credential…
pi-auth-logout = Log out…
pi-auth-remove-title = Remove credentials for “{ $provider }”?
pi-auth-logout-title = Log out of “{ $provider }”?
pi-auth-remove-help = Only local credentials are deleted. Server tokens are not revoked. Environment variables or apiKey in models.json may still provide authentication.
pi-auth-remove-env = Provider environment and configuration values will also be removed.
pi-auth-remove-confirm = Remove
pi-auth-logout-confirm = Log out
pi-auth-replace-title = Replace credentials for “{ $provider }”?
pi-auth-replace-help = The entire stored credential, including tokens and unknown fields, will be replaced with an API key.
pi-auth-replace-confirm = Replace
pi-auth-read-error = Could not read auth.json.
pi-auth-parse-error = auth.json must contain a valid JSON object. Editing is disabled.
pi-auth-busy = auth.json is busy. Try again.
pi-auth-conflict = This provider changed on disk. Reload and try again.
pi-auth-write-error = Could not write auth.json. Reload to inspect the current state.
pi-auth-input-error = Choose a provider or enter a nonempty single-line ID, and enter an API key.
pi-auth-login-select-help = Login command copied. Run /login in the Pi terminal and select the provider.

# Authentication and model provider settings
pi-files-auth-stored = Credentials stored in Authentication
pi-files-manage-auth = Manage in Authentication
pi-files-api-key-auth-help = When credentials are stored in Authentication, Pi does not fall back to this apiKey. Stored credentials do not mean authentication is valid.
pi-auth-models-help = Stored credentials affect how Pi uses apiKey in models.json. Pi handles authentication according to provider rules.
pi-auth-edit-models = Edit in Models & providers

pi-provider-empty-id = ⟨Empty ID⟩
pi-auth-check-unavailable = Pi cannot check this provider ID.

settings-skill-name-help = Directory name. Pi recommends lowercase letters, digits and single hyphens, up to 64 characters.

settings-skill-name-warning = This name is outside Pi’s recommendations; creation is still allowed. This does not validate the name in SKILL.md.

settings-prompt-name-help = Used as /name. Whitespace is not allowed.

settings-prompt-invalid-name = Use a valid filename without whitespace or path separators.

shortcut-add-title = Add template task

shortcut-edit-title = Edit template task

shortcut-options-failed = Could not load template or model options. Check the Pi connection and retry.

pi-auth-row-action = { $provider }: { $action }

pi-auth-add-title = Add API key

pi-auth-replace-editor-title = Replace API key

temporary-clean-title = Clean released working directories?

settings-package-remove-title = Remove “{ $name }”?

settings-resource-delete-title = Move “{ $name }” to Trash?

shortcut-delete-title = Delete template task “{ $name }”?

settings-key-control-label = { $command }: { $binding }
settings-key-control-recording = { $command }: { $binding } — { $state }

# MCP CLI login and last-check results
pi-mcp-help = This checks and signs in to Pi’s built-in MCP. Extensions replacing it may have different session states. Services using auth.provider may need login in this check but work in sessions.
pi-mcp-reconnect = To load saved MCP configuration fully in a running session, choose Reconnect in its conversation menu. New sessions load it automatically.
pi-mcp-global-cwd = Global checks run in an empty temporary directory; relative paths may behave differently in sessions.
pi-mcp-save-first = Save or discard changes first
pi-mcp-check = Check connection
pi-mcp-running = Working…
pi-mcp-last-check = Last check { $time }
pi-mcp-config-error = Configuration has errors
pi-mcp-untrusted = Project is not trusted; Pi uses only global configuration
pi-mcp-resources = Resources available
pi-mcp-connected = { $count ->
    [one] Connected ({ $count } tool)
   *[other] Connected ({ $count } tools)
    }
pi-mcp-needs-auth = Login required
pi-mcp-connection-failed = Connection failed
pi-mcp-disconnected = Not connected
pi-mcp-not-checked = Not checked
pi-mcp-unsupported-name = Pi cannot log in or out with this name through the CLI
pi-mcp-login = Log in
pi-mcp-logout = Log out
pi-mcp-login-title = Log in to MCP server “{ $name }”
pi-mcp-logout-title = Log out of MCP server “{ $name }”?
pi-mcp-login-help = If login is needed, Pi opens a browser. Complete login there.
pi-mcp-logout-help = Deletes locally saved credentials without revoking server authorization or immediately disconnecting running sessions. Reconnect those sessions to refresh their connections.
pi-mcp-login-terminal = If no browser appears, cancel and run pi mcp login -- { $name } in a terminal.
pi-mcp-completed = Command completed
pi-mcp-cancelled = Cancelled
pi-mcp-timeout = Operation timed out; results are incomplete
pi-mcp-login-failed = Login failed. Credentials may have been saved before the connection failed. Check the connection.
pi-mcp-failed = Operation failed
pi-mcp-close = Close
pi-mcp-scope-project = Project
pi-mcp-disabled = Disabled
pi-mcp-check-failed = Check failed
pi-mcp-check-timeout = Check timed out; results are incomplete

workspace-changes = Changes
workspace-close-tab = Close tab
workspace-close-others = Close other tabs
workspace-close-right = Close tabs to the right
workspace-toggle-file-tree = Toggle file tree
workspace-toggle-change-list = Toggle changed files
workspace-out-of-scope = This reply is outside the current scope.
workspace-net-zero = All changes in this reply cancel out.
workspace-open-file = Open file
workspace-next-operations = Next operations
workspace-open-change = Open { $path } in the workspace

history-panel-title = History
history-panel-options = History options
workspace-view-changes = View changes

workspace-new-tab = New tab
workspace-all-tabs = All tabs
workspace-tools = Tools
workspace-recent = Recent
workspace-no-changes = No changes in this conversation yet.
workspace-recent-unavailable = These changes are unavailable in the current scope.
workspace-just-now = Just now
workspace-minutes-ago = { $count } min ago
workspace-maximize = Maximize workspace
workspace-restore = Restore workspace

# Direct Shell command output
shell-title = Shell
shell-running = Running command
shell-stopping = Stopping command
shell-stop = Stop command
shell-cancelled = Cancelled
shell-exit-code = Exit code { $code }
shell-completed = Completed · exit status unavailable
shell-excluded = Not included in model context
shell-show-all = Show all output
shell-show-less = Show less output
shell-open-full-output = Open full output
shell-display-truncated = Only the most recent output is shown.

titlebar-git-branch = Git branch: { $name }
titlebar-git-detached = Detached HEAD: { $name }

# Diff reader
diff-worktree = Git
diff-unstaged = Unstaged
diff-staged = Staged
diff-untracked = Untracked
diff-filter = Filter paths…
diff-previous-file = Previous file
diff-next-file = Next file
diff-unified = Unified
diff-split = Side by side
diff-wrap = Wrap lines
diff-actions = File actions
diff-word = Word differences
diff-context = All unchanged lines
diff-expand-file = Expand this file
diff-copy-path = Copy file path
diff-copy-patch = Copy patch
diff-copy-content = Copy full file text
diff-external = Open in external editor
diff-refresh = Refresh changes
diff-refresh-failed = Couldn’t refresh changes. Previous results may be outdated.
diff-loading = Loading changes…
diff-not-repository = This folder is not in a Git worktree
diff-empty = No changes
diff-no-results = No matching files
diff-retry = Retry
diff-recorded-context = Recorded changes · only recorded context is available
diff-added = Added
diff-deleted = Deleted
diff-modified = Modified
diff-conflict = Conflict · read-only
diff-special = Special file · read-only
diff-binary = Binary file
diff-too-large = This file exceeds the text reader limits
diff-unavailable-file = The current file is unavailable; external open is disabled.
diff-git-unavailable = Git is unavailable. Install Git and refresh.
diff-files = Files
diff-conflicts = Conflicts
diff-direction-staged = HEAD → index
diff-direction-unstaged = index → worktree
diff-direction-untracked = empty → worktree
diff-updating = Updating previous version…
diff-collapse-file = Collapse this file
diff-update-failed = Couldn’t update this version
diff-reading-options = Reading options
diff-reading-options-narrow = Reading options: the content area is too narrow for split view; widening restores your preferred mode
diff-view = View
diff-display = Display
diff-list-view = List
diff-tree-view = Tree
diff-stats-incomplete = Statistics incomplete

workspace-back-conversation = Back to conversation
workspace-back-content = Back to content
workspace-open = Open workspace
workspace-close = Close workspace

diff-invalid-path = The file name is not valid UTF-8 and cannot be read
diff-no-text-changes = No text changes

diff-external-failed = Could not open the file. It may no longer exist, or no application can open it.

diff-conflict-label = Conflict
diff-read-failed = Couldn’t read changes. Retry.

# Conversation navigation
conversation-navigation-jump = Jump to user message { $number }: { $question }
conversation-navigation-current = Current interval. Jump to user message { $number }: { $question }
conversation-navigation-no-text = No text content

error-technical-details = Technical details
conversation-load-failed = Could not load conversation content.
conversation-refresh-failed = Could not refresh conversation content. Displayed content may be outdated.
history-load-failed = Could not load conversation history.
history-refresh-failed = Could not refresh conversation history. Displayed history may be outdated.
conversation-runtime-failed = Pi encountered a runtime problem.
conversation-storage-failed = Could not access local conversation data. See details and check the file location and permissions.
composer-model-load-failed = Could not load models.
composer-thinking-load-failed = Could not load thinking options.
composer-model-change-failed = Could not confirm the model or thinking-level change.
command-load-failed = Could not load commands.
resource-files-failed = Could not search for files. Check the path and search again.

quote-in-composer = Quote in composer
quote-select-text = Select text to quote.
quote-select-diff-lines = Select line numbers on one side of the diff.
quote-composer-unavailable = The normal message draft cannot be edited right now.
quote-message-title =
    { $role ->
        [user] User — Content at selection:
       *[assistant] Assistant — Content at selection:
    }
quote-file-title = File — Content at selection: { $path }, lines { $start }–{ $end }
quote-turn-title = Turn changes — Content at selection: { $path }, { $side ->
        [original] original side
       *[modified] modified side
    }, lines { $start }–{ $end }
quote-staged-title = Git · Staged — Content at selection: { $path }, { $side ->
        [original] original side
       *[modified] modified side
    }, lines { $start }–{ $end }
quote-unstaged-title = Git · Unstaged — Content at selection: { $path }, { $side ->
        [original] original side
       *[modified] modified side
    }, lines { $start }–{ $end }
quote-untracked-title = Git · Untracked — Content at selection: { $path }, { $side ->
        [original] original side
       *[modified] modified side
    }, lines { $start }–{ $end }
quote-disabled-menu =
    { $reason ->
        [selection] Quote in composer — select text
        [diff] Quote in composer — select line numbers on one side
       *[composer] Quote in composer — message draft unavailable
    }

quote-action = Quote
quote-comment-placeholder = Add a comment (optional)…
quote-comment-prefix = { "Comment: " }
quote-message-short =
    { $role ->
        [user] Your message
       *[assistant] Assistant reply
    }
quote-file-short = { $name } · lines { $start }–{ $end }
quote-diff-short = { $name } · { $side ->
        [original] original side
       *[modified] modified side
    } · { $start }–{ $end } lines
quote-command-unavailable = This command does not support quotes yet. Rewrite it as a normal message, or remove the quotes before sending.

quote-count =
    { $count ->
        [one] { $count } quote
       *[other] { $count } quotes
    }
quote-selected-text = Selected text:
quote-list-source = { $source }:
quote-user-comment = User comment:
quote-edit-comment = Edit comment
quote-delete-number = Delete quote { $number }: { $source }
quote-clear = Clear quotes
quote-edit-label = Edit comment { $number }: { $source }
