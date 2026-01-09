# tui-app Technical Specification

## Overview

**Crate Name**: `tui-app`
**Purpose**: Application orchestration, event loop, protocol handling, state management
**Dependencies**: `tui-core`, `tui-render`, `tui-terminal`, `tui-cells`, `tui-input`, `tokio`
**Dependents**: None (top-level application)

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: Application Lifecycle
- FR-1.1: Initialize terminal and enter TUI mode
- FR-1.2: Run main event loop
- FR-1.3: Handle graceful shutdown
- FR-1.4: Restore terminal on exit (normal or panic)
- FR-1.5: Support configuration via CLI arguments
- FR-1.6: Support configuration via environment variables

#### FR-2: Event Loop
- FR-2.1: Process terminal events (key, paste, resize, focus)
- FR-2.2: Process protocol events from backend
- FR-2.3: Process frame requests (redraw)
- FR-2.4: Support event loop pause/resume (for external programs)
- FR-2.5: Handle interrupt signals (Ctrl+C)
- FR-2.6: Support background task completion events

#### FR-3: Application State
- FR-3.1: Track current view/screen
- FR-3.2: Track conversation history (cells)
- FR-3.3: Track streaming state
- FR-3.4: Track pending approvals
- FR-3.5: Track active modals
- FR-3.6: Track session metadata

#### FR-4: View Management
- FR-4.1: Welcome/onboarding screen
- FR-4.2: Main chat view
- FR-4.3: Session picker view
- FR-4.4: Settings/configuration view
- FR-4.5: Modal overlay system
- FR-4.6: View transitions

#### FR-5: Protocol Bridge
- FR-5.1: Receive events from backend protocol
- FR-5.2: Dispatch events to appropriate handlers
- FR-5.3: Send operations to backend
- FR-5.4: Handle protocol errors gracefully
- FR-5.5: Support multiple message types (agent, tool, exec, etc.)
- FR-5.6: Handle rate limiting

#### FR-6: History Management
- FR-6.1: Add cells to history
- FR-6.2: Update existing cells (streaming)
- FR-6.3: Scroll history view
- FR-6.4: Calculate visible cells
- FR-6.5: Export history (transcript)
- FR-6.6: Clear history

#### FR-7: Approval Workflow
- FR-7.1: Queue approval requests
- FR-7.2: Show approval modal
- FR-7.3: Handle approve/reject responses
- FR-7.4: Support "always approve" rules
- FR-7.5: Handle deferred approvals during streaming

#### FR-8: External Program Execution
- FR-8.1: Suspend TUI for external programs
- FR-8.2: Launch external editor
- FR-8.3: Launch pager for long output
- FR-8.4: Resume TUI after external program
- FR-8.5: Capture external program output (if needed)

#### FR-9: Onboarding
- FR-9.1: Authentication flow
- FR-9.2: Trust directory configuration
- FR-9.3: Welcome screen with instructions
- FR-9.4: Account/rate limit information
- FR-9.5: First-run detection

#### FR-10: Session Management
- FR-10.1: Create new sessions
- FR-10.2: Resume previous sessions
- FR-10.3: Session picker with history
- FR-10.4: Session logging
- FR-10.5: Session metadata display

#### FR-11: Notifications
- FR-11.1: Desktop notifications (when unfocused)
- FR-11.2: In-app status updates
- FR-11.3: Error notifications
- FR-11.4: Completion notifications

#### FR-12: Clipboard Integration
- FR-12.1: Handle paste events
- FR-12.2: Support image paste
- FR-12.3: Handle large paste (burst detection)

### 1.2 Non-Functional Requirements

- NFR-1: Single-threaded event loop (no blocking operations)
- NFR-2: All I/O MUST be async
- NFR-3: Frame rate MUST be configurable (default 30fps)
- NFR-4: Startup time < 500ms to first frame
- NFR-5: Memory usage scales with history size
- NFR-6: Graceful degradation on terminal capability mismatch

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Backend protocol implementation (uses trait from tui-core)
- NR-2: Widget rendering logic (uses tui-cells, tui-input)
- NR-3: Text rendering (uses tui-render)
- NR-4: Terminal I/O (uses tui-terminal)
- NR-5: Multi-window support
- NR-6: Plugin system

---

## 3. Interface Specification

### 3.1 Application Entry Point

```rust
pub struct AppConfig {
    pub initial_prompt: Option<String>,
    pub working_dir: PathBuf,
    pub context_files: Vec<PathBuf>,
    pub context_urls: Vec<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub frame_rate: u32,
    pub inline_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self;
}

pub struct App<P: Protocol> {
    // Private fields
}

impl<P: Protocol> App<P> {
    pub async fn new(config: AppConfig, protocol: P) -> io::Result<Self>;
    pub async fn run(self) -> io::Result<AppExitReason>;
}

pub enum AppExitReason {
    UserQuit,
    SessionEnd,
    Error(String),
}
```

### 3.2 Protocol Trait

```rust
pub trait Protocol: Send + 'static {
    type Event: ProtocolEvent;
    type Op: ProtocolOp;

    fn event_stream(&self) -> impl Stream<Item = Self::Event> + Send;
    fn send(&self, op: Self::Op) -> impl Future<Output = Result<(), ProtocolError>> + Send;
}

#[derive(Debug)]
pub struct ProtocolError {
    pub kind: ProtocolErrorKind,
    pub message: String,
}

pub enum ProtocolErrorKind {
    Connection,
    Timeout,
    InvalidResponse,
    RateLimit,
    Auth,
    Other,
}
```

### 3.3 Application State

```rust
pub struct AppState {
    pub view: AppView,
    pub history: Vec<Box<dyn CellData>>,
    pub stream_controller: StreamController,
    pub pending_approvals: VecDeque<ApprovalRequest>,
    pub active_modal: Option<Box<dyn ModalView>>,
    pub session: Option<SessionInfo>,
    pub config: RuntimeConfig,
}

pub enum AppView {
    Welcome,
    Chat,
    SessionPicker,
    Settings,
    Onboarding(OnboardingStep),
}

pub enum OnboardingStep {
    Auth,
    TrustDirectory,
    Welcome,
}

pub struct SessionInfo {
    pub id: String,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub working_dir: PathBuf,
}

pub struct RuntimeConfig {
    pub model: String,
    pub auto_approve: Vec<ApprovalRule>,
    pub inline_mode: bool,
    pub frame_rate: u32,
}
```

### 3.4 Event Handling

```rust
pub struct EventLoop<P: Protocol> {
    tui: Tui,
    protocol: P,
    state: AppState,
    handlers: EventHandlers,
}

pub struct EventHandlers {
    pub lifecycle: LifecycleHandler,
    pub protocol: ProtocolEventHandler,
    pub ui: UiEventHandler,
}

impl<P: Protocol> EventLoop<P> {
    pub fn new(tui: Tui, protocol: P, state: AppState) -> Self;
    pub async fn run(mut self) -> io::Result<AppExitReason>;
}

// Handler trait for event categories
pub trait EventHandler<E> {
    fn handle(&mut self, event: E, state: &mut AppState) -> Vec<AppAction>;
}

pub enum AppAction {
    Render,
    SendProtocol(Box<dyn ProtocolOp>),
    ShowModal(Box<dyn ModalView>),
    DismissModal,
    AddCell(Box<dyn CellData>),
    UpdateCell(usize),
    Exit(AppExitReason),
    LaunchExternal(ExternalProgram),
    Notify(String),
}
```

### 3.5 View Rendering

```rust
pub struct ChatView {
    history_scroll: ScrollState,
    composer: Composer,
    bottom_pane: BottomPane,
}

impl ChatView {
    pub fn new() -> Self;
    pub fn render(&self, state: &AppState, area: Rect, buf: &mut Buffer);
    pub fn handle_key(&mut self, key: KeyEvent, state: &mut AppState) -> Vec<AppAction>;
}

pub struct BottomPane {
    composer: Composer,
    footer: Footer,
}

pub struct Footer {
    hints: Vec<KeyHint>,
    status: Option<String>,
}

pub struct KeyHint {
    pub key: String,
    pub description: String,
}
```

### 3.6 History Management

```rust
pub struct HistoryManager {
    cells: Vec<Box<dyn CellData>>,
    renderers: CellRendererRegistry,
}

impl HistoryManager {
    pub fn new() -> Self;
    pub fn push(&mut self, cell: Box<dyn CellData>);
    pub fn get(&self, index: usize) -> Option<&dyn CellData>;
    pub fn get_mut(&mut self, index: usize) -> Option<&mut dyn CellData>;
    pub fn len(&self) -> usize;
    pub fn clear(&mut self);

    pub fn render_cell(&self, index: usize, width: u16) -> Vec<Line<'static>>;
    pub fn total_height(&self, width: u16) -> usize;
    pub fn visible_range(&self, scroll: &ScrollState, height: u16, width: u16) -> Range<usize>;

    pub fn export_transcript(&self, width: u16) -> String;
}

pub struct CellRendererRegistry {
    renderers: HashMap<TypeId, Box<dyn Any>>,
}
```

### 3.7 Approval System

```rust
pub struct ApprovalRequest {
    pub id: String,
    pub kind: ApprovalKind,
    pub description: String,
    pub details: Option<String>,
    pub allow_always: bool,
    pub allow_edit: bool,
}

pub enum ApprovalKind {
    Command { command: String, cwd: PathBuf },
    FileWrite { path: PathBuf },
    FileDelete { path: PathBuf },
    NetworkRequest { url: String },
    Custom { type_name: String },
}

pub struct ApprovalRule {
    pub pattern: ApprovalPattern,
    pub action: ApprovalAction,
}

pub enum ApprovalPattern {
    CommandPrefix(String),
    FilePath(PathBuf),
    All,
}

pub enum ApprovalAction {
    Approve,
    Reject,
    Ask,
}

pub struct ApprovalManager {
    pending: VecDeque<ApprovalRequest>,
    rules: Vec<ApprovalRule>,
    deferred: Vec<ApprovalRequest>,
}

impl ApprovalManager {
    pub fn queue(&mut self, request: ApprovalRequest);
    pub fn next_pending(&mut self) -> Option<ApprovalRequest>;
    pub fn respond(&mut self, id: &str, response: ApprovalResponse);
    pub fn add_rule(&mut self, rule: ApprovalRule);
    pub fn check_auto_approve(&self, request: &ApprovalRequest) -> Option<ApprovalAction>;
    pub fn defer(&mut self, request: ApprovalRequest);
    pub fn restore_deferred(&mut self);
}

pub enum ApprovalResponse {
    Approved,
    Rejected,
    ApprovedAlways,
    Edit(String),
}
```

### 3.8 External Program Support

```rust
pub enum ExternalProgram {
    Editor { path: PathBuf, line: Option<usize> },
    Pager { content: String },
    Command { command: String, cwd: PathBuf },
}

pub struct ExternalRunner {
    // Private
}

impl ExternalRunner {
    pub fn new() -> Self;
    pub async fn run(&self, program: ExternalProgram, tui: &mut Tui) -> io::Result<ExternalResult>;
}

pub enum ExternalResult {
    Success,
    Cancelled,
    Error(String),
    EditorContent(String),
}
```

### 3.9 Onboarding

```rust
pub struct OnboardingFlow {
    step: OnboardingStep,
    auth_state: Option<AuthState>,
    trust_dirs: Vec<PathBuf>,
}

pub enum AuthState {
    NotAuthenticated,
    Authenticating,
    Authenticated { user: String },
    Error(String),
}

impl OnboardingFlow {
    pub fn new() -> Self;
    pub fn needs_onboarding(&self) -> bool;
    pub fn current_step(&self) -> OnboardingStep;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
    pub fn handle_key(&mut self, key: KeyEvent) -> OnboardingAction;
}

pub enum OnboardingAction {
    Continue,
    Skip,
    Complete,
    LaunchBrowser(String),
    SetTrustDir(PathBuf),
}
```

### 3.10 Session Management

```rust
pub struct SessionManager {
    current: Option<SessionInfo>,
    log: Option<SessionLog>,
}

pub struct SessionLog {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl SessionManager {
    pub fn new() -> Self;
    pub fn start(&mut self, session: SessionInfo, log_dir: Option<PathBuf>) -> io::Result<()>;
    pub fn end(&mut self) -> io::Result<()>;
    pub fn current(&self) -> Option<&SessionInfo>;
    pub fn log_event(&mut self, event: &str) -> io::Result<()>;
}

pub struct SessionPicker {
    sessions: Vec<SessionSummary>,
    selected: usize,
}

pub struct SessionSummary {
    pub id: String,
    pub preview: String,
    pub timestamp: DateTime<Utc>,
    pub message_count: usize,
}

impl SessionPicker {
    pub fn new(sessions: Vec<SessionSummary>) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
    pub fn handle_key(&mut self, key: KeyEvent) -> SessionPickerResult;
}

pub enum SessionPickerResult {
    Selected(String),
    NewSession,
    Cancelled,
    Pending,
}
```

---

## 4. Acceptance Criteria

### AC-1: Application Lifecycle
- [ ] AC-1.1: `App::new()` initializes terminal
- [ ] AC-1.2: `App::run()` starts event loop
- [ ] AC-1.3: Exit via quit command restores terminal
- [ ] AC-1.4: Panic during run restores terminal
- [ ] AC-1.5: Ctrl+C triggers graceful shutdown
- [ ] AC-1.6: Exit reason is correctly reported

### AC-2: Event Loop
- [ ] AC-2.1: Key events reach input handler
- [ ] AC-2.2: Protocol events update state
- [ ] AC-2.3: Frame requests trigger render
- [ ] AC-2.4: Resize events update layout
- [ ] AC-2.5: Focus events update notification state
- [ ] AC-2.6: Event loop can be paused/resumed

### AC-3: State Management
- [ ] AC-3.1: View transitions update `AppView`
- [ ] AC-3.2: History cells are stored in order
- [ ] AC-3.3: Streaming state tracks active cell
- [ ] AC-3.4: Modal stack works correctly
- [ ] AC-3.5: Session info persists during session

### AC-4: Chat View
- [ ] AC-4.1: History scrolls with Up/Down/PageUp/PageDown
- [ ] AC-4.2: Composer handles text input
- [ ] AC-4.3: Enter submits message
- [ ] AC-4.4: Slash commands show popup
- [ ] AC-4.5: Footer shows relevant key hints
- [ ] AC-4.6: Status updates display in footer

### AC-5: Protocol Bridge
- [ ] AC-5.1: Agent messages create/update cells
- [ ] AC-5.2: Tool calls create tool cells
- [ ] AC-5.3: Exec commands create exec cells
- [ ] AC-5.4: Errors create error cells
- [ ] AC-5.5: Rate limit errors handled gracefully
- [ ] AC-5.6: Connection errors show error state

### AC-6: History
- [ ] AC-6.1: New cells appear at bottom
- [ ] AC-6.2: Streaming updates correct cell
- [ ] AC-6.3: Scroll position maintained during updates
- [ ] AC-6.4: Auto-scroll to bottom on new content (if at bottom)
- [ ] AC-6.5: Transcript export includes all cells
- [ ] AC-6.6: Clear removes all cells

### AC-7: Approvals
- [ ] AC-7.1: Approval requests show modal
- [ ] AC-7.2: Approve sends approval to protocol
- [ ] AC-7.3: Reject sends rejection to protocol
- [ ] AC-7.4: Always approve adds rule
- [ ] AC-7.5: Deferred approvals restored after streaming
- [ ] AC-7.6: Auto-approve rules checked first

### AC-8: External Programs
- [ ] AC-8.1: Editor launch restores terminal
- [ ] AC-8.2: Editor content captured on close
- [ ] AC-8.3: Pager shows content
- [ ] AC-8.4: TUI resumes after external program
- [ ] AC-8.5: Input flushed after external program

### AC-9: Onboarding
- [ ] AC-9.1: First run shows welcome
- [ ] AC-9.2: Auth required before chat
- [ ] AC-9.3: Auth launches browser
- [ ] AC-9.4: Trust directory configurable
- [ ] AC-9.5: Onboarding can be skipped

### AC-10: Notifications
- [ ] AC-10.1: Notifications sent when unfocused
- [ ] AC-10.2: No notifications when focused
- [ ] AC-10.3: Notifications suppressed when disabled

---

## 5. Testing Requirements

### 5.1 Unit Tests - Event Handlers

```
test_lifecycle_handler_exit
test_lifecycle_handler_new_session
test_protocol_handler_agent_message
test_protocol_handler_tool_call
test_protocol_handler_exec_command
test_protocol_handler_error
test_ui_handler_key_char
test_ui_handler_key_escape
test_ui_handler_modal_dismiss
```

### 5.2 Unit Tests - State Management

```
test_state_view_transition
test_state_add_cell
test_state_update_cell
test_state_modal_push
test_state_modal_pop
test_state_modal_stack
```

### 5.3 Unit Tests - History

```
test_history_push
test_history_get
test_history_get_mut
test_history_clear
test_history_visible_range_small
test_history_visible_range_scrolled
test_history_total_height
test_history_export_transcript
```

### 5.4 Unit Tests - Approvals

```
test_approval_queue
test_approval_next_pending
test_approval_respond
test_approval_auto_approve_match
test_approval_auto_approve_no_match
test_approval_defer
test_approval_restore_deferred
test_approval_rule_command_prefix
test_approval_rule_file_path
```

### 5.5 Unit Tests - Session

```
test_session_start
test_session_end
test_session_log_event
test_session_picker_select
test_session_picker_new
test_session_picker_navigate
```

### 5.6 Unit Tests - Onboarding

```
test_onboarding_needs_onboarding
test_onboarding_step_progression
test_onboarding_skip
test_onboarding_complete
test_onboarding_auth_state_transitions
```

### 5.7 Integration Tests

```
test_app_startup_shutdown
test_app_submit_message
test_app_receive_response
test_app_approval_flow
test_app_external_editor
test_app_session_resume
test_event_loop_key_to_cell
test_event_loop_protocol_to_cell
```

### 5.8 End-to-End Tests (with mock protocol)

```
test_e2e_simple_conversation
test_e2e_tool_call_approved
test_e2e_tool_call_rejected
test_e2e_streaming_response
test_e2e_error_recovery
test_e2e_session_persistence
```

### 5.9 Stress Tests

```
test_stress_rapid_messages
test_stress_large_history
test_stress_long_streaming
test_stress_many_approvals
```

---

## 6. Error Handling

### 6.1 Error Recovery Strategy

| Error Type | Recovery |
|------------|----------|
| Protocol connection lost | Show error cell, attempt reconnect |
| Protocol timeout | Show error, allow retry |
| Rate limit hit | Show warning, queue request |
| Invalid protocol response | Log error, show generic error cell |
| Render error | Log, skip frame |
| Terminal I/O error | Attempt restore, exit |

### 6.2 User-Facing Errors

| Situation | User Feedback |
|-----------|---------------|
| Auth failure | Show auth error in onboarding |
| Session resume failure | Show error, offer new session |
| External program failure | Show error cell |
| Approval timeout | Re-queue approval |

### 6.3 Error Types

```rust
pub enum AppError {
    Terminal(io::Error),
    Protocol(ProtocolError),
    Session(SessionError),
    Config(ConfigError),
}

pub enum SessionError {
    NotFound(String),
    LoadFailed(io::Error),
    SaveFailed(io::Error),
}

pub enum ConfigError {
    ParseFailed(String),
    InvalidValue { key: String, value: String },
}
```

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tui-core` | workspace | Traits, primitives |
| `tui-render` | workspace | Text rendering |
| `tui-terminal` | workspace | Terminal I/O |
| `tui-cells` | workspace | History cells |
| `tui-input` | workspace | Input widgets |
| `tokio` | 1.0+ | Async runtime |
| `tokio-stream` | 0.1+ | Stream utilities |
| `futures` | 0.3+ | Future utilities |
| `chrono` | 0.4+ | Timestamps |
| `clap` | 4.0+ | CLI parsing |
| `serde` | 1.0+ | Config serialization |
| `serde_json` | 1.0+ | JSON handling |
| `tracing` | 0.1+ | Logging |
| `dirs` | 5.0+ | Standard directories |

### 7.2 Optional Dependencies

| Crate | Feature | Purpose |
|-------|---------|---------|
| `arboard` | clipboard | Clipboard access |
| `open` | browser | Open URLs in browser |

### 7.3 Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio-test` | Async test utilities |
| `mockall` | Mocking for tests |
| `insta` | Snapshot testing |
| `proptest` | Property-based testing |

---

## 8. File Structure

```
tui-app/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports
    ├── main.rs                 # Entry point
    ├── cli.rs                  # CLI argument parsing
    │
    ├── app/
    │   ├── mod.rs              # App struct
    │   ├── config.rs           # AppConfig
    │   ├── state.rs            # AppState
    │   └── actions.rs          # AppAction enum
    │
    ├── event_loop/
    │   ├── mod.rs              # EventLoop struct
    │   └── handlers/
    │       ├── mod.rs
    │       ├── lifecycle.rs    # Startup, shutdown
    │       ├── protocol.rs     # Protocol events
    │       └── ui.rs           # UI events
    │
    ├── views/
    │   ├── mod.rs
    │   ├── chat.rs             # ChatView
    │   ├── welcome.rs          # WelcomeView
    │   ├── settings.rs         # SettingsView
    │   └── bottom_pane.rs      # BottomPane, Footer
    │
    ├── history/
    │   ├── mod.rs              # HistoryManager
    │   └── renderer.rs         # CellRendererRegistry
    │
    ├── approval/
    │   ├── mod.rs              # ApprovalManager
    │   └── rules.rs            # ApprovalRule, patterns
    │
    ├── external/
    │   ├── mod.rs              # ExternalRunner
    │   ├── editor.rs           # Editor launch
    │   └── pager.rs            # Pager launch
    │
    ├── onboarding/
    │   ├── mod.rs              # OnboardingFlow
    │   ├── auth.rs             # Auth handling
    │   ├── trust.rs            # Trust directory
    │   └── welcome.rs          # Welcome screen
    │
    ├── session/
    │   ├── mod.rs              # SessionManager
    │   ├── log.rs              # SessionLog
    │   └── picker.rs           # SessionPicker
    │
    └── notify/
        └── mod.rs              # Notification dispatch
```

---

## 9. Thread Safety Requirements

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `App` | No | No | Owns terminal |
| `AppState` | No | No | Contains cells with internal mutability |
| `EventLoop` | No | No | Main thread only |
| `Protocol` trait | Yes | - | Required by trait bound |
| `ApprovalManager` | Yes | No | Contains VecDeque |
| `SessionManager` | No | No | Contains File handle |

---

## 10. Performance Requirements

- PR-1: Event loop iteration < 16ms for 60fps
- PR-2: Render frame < 10ms for responsive UI
- PR-3: History scroll MUST be smooth (no jank)
- PR-4: Startup to first frame < 500ms
- PR-5: Memory usage < 100MB base + O(history size)
- PR-6: Protocol event processing < 5ms per event

---

## 11. Configuration Schema

```toml
[tui]
frame_rate = 30
inline_mode = false

[session]
log_directory = "~/.local/share/tui/logs"
max_history = 1000

[auto_approve]
commands = ["ls", "cat", "echo"]
paths = ["./src/**", "./tests/**"]

[notifications]
enabled = true
sound = false

[editor]
command = "$EDITOR"
fallback = "vim"
```

---

## 12. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
