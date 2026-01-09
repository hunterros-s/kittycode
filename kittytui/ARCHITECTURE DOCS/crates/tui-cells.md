# tui-cells Technical Specification

## Overview

**Crate Name**: `tui-cells`
**Purpose**: History cell data types, renderers, and streaming state management
**Dependencies**: `tui-core`, `tui-render`, `ratatui`, `tokio`
**Dependents**: `tui-app`

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: User Message Cells
- FR-1.1: Store user-authored text content
- FR-1.2: Support image attachments with metadata
- FR-1.3: Support optional timestamps
- FR-1.4: Render with distinct user styling
- FR-1.5: Support transcript export (text only, no images)

#### FR-2: Agent Message Cells
- FR-2.1: Store agent response text
- FR-2.2: Support streaming state (incomplete content)
- FR-2.3: Support markdown rendering with optional wrapping
- FR-2.4: Track thinking/reasoning blocks separately
- FR-2.5: Support tool use indicators during streaming
- FR-2.6: Render shimmer effect during active streaming
- FR-2.7: Display cost/token information when available

#### FR-3: Tool Call Cells
- FR-3.1: Store tool name and parameters
- FR-3.2: Store tool result/output
- FR-3.3: Support collapsible display
- FR-3.4: Track execution state (pending, running, complete, error)
- FR-3.5: Support multiple tool types (MCP, exec, patch, etc.)

#### FR-4: Exec Command Cells
- FR-4.1: Store command string
- FR-4.2: Store working directory
- FR-4.3: Store output (stdout/stderr combined or separate)
- FR-4.4: Store exit code
- FR-4.5: Support truncated output with expansion
- FR-4.6: Track execution duration
- FR-4.7: Support approval state (pending, approved, rejected)

#### FR-5: Patch Cells
- FR-5.1: Store file path
- FR-5.2: Store patch content (unified diff format)
- FR-5.3: Store patch type (create, modify, delete, rename)
- FR-5.4: Support collapsible diff display
- FR-5.5: Show addition/deletion statistics
- FR-5.6: Support approval state

#### FR-6: Session Info Cells
- FR-6.1: Store session metadata (model, config, etc.)
- FR-6.2: Store working directory
- FR-6.3: Store context files/URLs
- FR-6.4: Support key-value display format
- FR-6.5: Builder pattern for construction

#### FR-7: System Cells
- FR-7.1: Info cells for status messages
- FR-7.2: Warning cells for non-critical issues
- FR-7.3: Error cells for failures with optional details
- FR-7.4: Plan update cells for todo/task changes

#### FR-8: Streaming Support
- FR-8.1: Controller for managing streaming state
- FR-8.2: Append text to streaming cells
- FR-8.3: Finalize streaming cells when complete
- FR-8.4: Track which cell is currently streaming
- FR-8.5: Support cancellation of streaming
- FR-8.6: Provide animation tick for shimmer effects

#### FR-9: Cell Registry
- FR-9.1: Register cell types for downcasting
- FR-9.2: Support dynamic cell type discovery
- FR-9.3: Provide type-safe downcast helpers

### 1.2 Non-Functional Requirements

- NFR-1: All CellData types MUST be Send + Sync
- NFR-2: All renderers MUST produce `Vec<Line<'static>>`
- NFR-3: Streaming updates MUST be O(1) append (no re-render)
- NFR-4: Memory-efficient storage for large outputs
- NFR-5: Renderers MUST handle width=0 gracefully
- NFR-6: All public types MUST be documented

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Persistence/serialization of cells (app layer handles)
- NR-2: Network communication for streaming
- NR-3: Cell layout/positioning (app layer handles)
- NR-4: Scroll management (app layer handles)
- NR-5: Cell selection/interaction (app layer handles)
- NR-6: Protocol-specific types (use generic traits from tui-core)

---

## 3. Interface Specification

### 3.1 User Message Types

```rust
pub struct UserMessageData {
    text: String,
    images: Vec<ImageAttachment>,
    timestamp: Option<DateTime<Utc>>,
}

pub struct ImageAttachment {
    pub path: PathBuf,
    pub media_type: String,
    pub size_bytes: Option<u64>,
}

impl CellData for UserMessageData {
    fn category(&self) -> CellCategory;
    fn text_content(&self) -> Cow<'_, str>;
    fn is_continuation(&self) -> bool;
    fn is_active(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct UserMessageRenderer {
    style: Style,
}

impl CellRenderer<UserMessageData> for UserMessageRenderer {
    fn render_lines(&self, cell: &UserMessageData, width: u16) -> Vec<Line<'static>>;
    fn transcript_lines(&self, cell: &UserMessageData, width: u16) -> Vec<Line<'static>>;
    fn height(&self, cell: &UserMessageData, width: u16) -> u16;
}
```

### 3.2 Agent Message Types

```rust
pub struct AgentMessageData {
    content: String,
    thinking: Option<String>,
    is_streaming: bool,
    tool_use_active: bool,
    cost: Option<CostInfo>,
}

pub struct CostInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: Option<f64>,
}

impl AgentMessageData {
    pub fn new() -> Self;
    pub fn append(&mut self, text: &str);
    pub fn set_thinking(&mut self, thinking: String);
    pub fn finalize(&mut self);
    pub fn is_streaming(&self) -> bool;
    pub fn set_tool_use_active(&mut self, active: bool);
}

pub struct AgentMessageRenderer {
    styles: AgentStyles,
    shimmer_phase: u8,
}

pub struct AgentStyles {
    pub text: Style,
    pub thinking: Style,
    pub tool_indicator: Style,
    pub cost: Style,
}

impl AgentMessageRenderer {
    pub fn tick_shimmer(&mut self);
}
```

### 3.3 Tool Call Types

```rust
pub struct ToolCallData {
    tool_name: String,
    parameters: serde_json::Value,
    result: Option<ToolResult>,
    state: ToolState,
    collapsed: bool,
}

pub enum ToolState {
    Pending,
    Running,
    Complete,
    Error(String),
}

pub enum ToolResult {
    Success(String),
    Error(String),
    Cancelled,
}

impl ToolCallData {
    pub fn new(name: String, params: serde_json::Value) -> Self;
    pub fn start(&mut self);
    pub fn complete(&mut self, result: ToolResult);
    pub fn toggle_collapsed(&mut self);
}
```

### 3.4 Exec Command Types

```rust
pub struct ExecCommandData {
    command: String,
    cwd: PathBuf,
    output: ExecOutput,
    exit_code: Option<i32>,
    duration: Option<Duration>,
    approval_state: ApprovalState,
    collapsed: bool,
}

pub enum ExecOutput {
    Empty,
    Content(String),
    Truncated { visible: String, full: String },
}

pub enum ApprovalState {
    NotRequired,
    Pending,
    Approved,
    Rejected,
}

impl ExecCommandData {
    pub fn new(command: String, cwd: PathBuf) -> Self;
    pub fn set_output(&mut self, output: String, max_lines: usize);
    pub fn complete(&mut self, exit_code: i32, duration: Duration);
    pub fn approve(&mut self);
    pub fn reject(&mut self);
    pub fn expand_output(&mut self);
    pub fn is_success(&self) -> bool;
}
```

### 3.5 Patch Types

```rust
pub struct PatchData {
    path: PathBuf,
    patch_type: PatchType,
    content: String,
    additions: usize,
    deletions: usize,
    approval_state: ApprovalState,
    collapsed: bool,
}

pub enum PatchType {
    Create,
    Modify,
    Delete,
    Rename { from: PathBuf },
}

impl PatchData {
    pub fn new(path: PathBuf, patch_type: PatchType, content: String) -> Self;
    pub fn toggle_collapsed(&mut self);
    pub fn approve(&mut self);
    pub fn reject(&mut self);
}
```

### 3.6 Session Info Types

```rust
pub struct SessionInfoData {
    entries: Vec<(String, String)>,
}

pub struct SessionInfoBuilder {
    entries: Vec<(String, String)>,
}

impl SessionInfoBuilder {
    pub fn new() -> Self;
    pub fn model(self, model: &str) -> Self;
    pub fn working_dir(self, path: &Path) -> Self;
    pub fn context_file(self, path: &Path) -> Self;
    pub fn context_url(self, url: &str) -> Self;
    pub fn custom(self, key: &str, value: &str) -> Self;
    pub fn build(self) -> SessionInfoData;
}
```

### 3.7 System Cell Types

```rust
pub struct InfoData {
    message: String,
}

pub struct WarningData {
    message: String,
}

pub struct ErrorData {
    message: String,
    details: Option<String>,
}

pub struct PlanUpdateData {
    items: Vec<PlanItem>,
}

pub struct PlanItem {
    pub text: String,
    pub state: PlanItemState,
}

pub enum PlanItemState {
    Pending,
    InProgress,
    Complete,
    Skipped,
}
```

### 3.8 Streaming Controller

```rust
pub struct StreamController {
    active_cell: Option<CellId>,
    shimmer_tick: u8,
}

pub type CellId = usize;

impl StreamController {
    pub fn new() -> Self;
    pub fn start_stream(&mut self, cell_id: CellId);
    pub fn append_text(&self, cell_id: CellId, text: &str, cells: &mut [Box<dyn CellData>]);
    pub fn finalize(&mut self, cell_id: CellId, cells: &mut [Box<dyn CellData>]);
    pub fn cancel(&mut self, cells: &mut [Box<dyn CellData>]);
    pub fn is_streaming(&self) -> bool;
    pub fn active_cell(&self) -> Option<CellId>;
    pub fn tick(&mut self) -> u8;
}
```

### 3.9 Cell Registry

```rust
pub struct CellRegistry {
    // Private
}

impl CellRegistry {
    pub fn new() -> Self;
    pub fn register<T: CellData + 'static>(&mut self);
    pub fn downcast<T: CellData + 'static>(cell: &dyn CellData) -> Option<&T>;
    pub fn downcast_mut<T: CellData + 'static>(cell: &mut dyn CellData) -> Option<&mut T>;
}

// Convenience functions
pub fn downcast_cell<T: CellData + 'static>(cell: &dyn CellData) -> Option<&T>;
pub fn downcast_cell_mut<T: CellData + 'static>(cell: &mut dyn CellData) -> Option<&mut T>;
```

---

## 4. Acceptance Criteria

### AC-1: User Message Cells
- [ ] AC-1.1: `UserMessageData` stores text content
- [ ] AC-1.2: `UserMessageData` stores image attachments
- [ ] AC-1.3: `text_content()` returns user text
- [ ] AC-1.4: `category()` returns `CellCategory::UserMessage`
- [ ] AC-1.5: Renderer produces styled lines
- [ ] AC-1.6: Transcript excludes image metadata

### AC-2: Agent Message Cells
- [ ] AC-2.1: `AgentMessageData::append()` adds text
- [ ] AC-2.2: `is_streaming()` reflects streaming state
- [ ] AC-2.3: `finalize()` marks streaming complete
- [ ] AC-2.4: Thinking blocks render with distinct style
- [ ] AC-2.5: Tool use indicator shows during tool execution
- [ ] AC-2.6: Shimmer animates during streaming
- [ ] AC-2.7: Cost info displays when available

### AC-3: Tool Call Cells
- [ ] AC-3.1: Tool name and parameters stored
- [ ] AC-3.2: State transitions: Pending → Running → Complete/Error
- [ ] AC-3.3: Results stored on completion
- [ ] AC-3.4: Collapsed state toggleable
- [ ] AC-3.5: Renderer shows appropriate state indicators

### AC-4: Exec Command Cells
- [ ] AC-4.1: Command string preserved exactly
- [ ] AC-4.2: Working directory stored
- [ ] AC-4.3: Output stored (respecting truncation limit)
- [ ] AC-4.4: Exit code reflects actual process result
- [ ] AC-4.5: Duration tracked from start to completion
- [ ] AC-4.6: Approval state transitions correctly
- [ ] AC-4.7: `is_success()` returns true only for exit code 0
- [ ] AC-4.8: Truncated output expandable

### AC-5: Patch Cells
- [ ] AC-5.1: File path stored
- [ ] AC-5.2: Patch content stored in unified diff format
- [ ] AC-5.3: Addition/deletion counts accurate
- [ ] AC-5.4: Renderer shows diff with syntax highlighting
- [ ] AC-5.5: Collapsed state toggleable
- [ ] AC-5.6: Approval state transitions correctly

### AC-6: Session Info Cells
- [ ] AC-6.1: Builder pattern constructs valid cells
- [ ] AC-6.2: Entries rendered as key-value pairs
- [ ] AC-6.3: Multiple context files supported
- [ ] AC-6.4: Custom entries supported

### AC-7: System Cells
- [ ] AC-7.1: Info cells render with info style
- [ ] AC-7.2: Warning cells render with warning style
- [ ] AC-7.3: Error cells render with error style
- [ ] AC-7.4: Error details expandable
- [ ] AC-7.5: Plan items show state indicators

### AC-8: Streaming
- [ ] AC-8.1: `start_stream()` sets active cell
- [ ] AC-8.2: `append_text()` updates correct cell
- [ ] AC-8.3: `finalize()` clears streaming state
- [ ] AC-8.4: `cancel()` finalizes and clears
- [ ] AC-8.5: `tick()` increments shimmer phase
- [ ] AC-8.6: Only one cell streams at a time

### AC-9: Registry
- [ ] AC-9.1: `downcast()` returns Some for correct type
- [ ] AC-9.2: `downcast()` returns None for wrong type
- [ ] AC-9.3: `downcast_mut()` allows mutation

### AC-10: CellData Trait Compliance
- [ ] AC-10.1: All cell types implement CellData
- [ ] AC-10.2: All cell types are Send + Sync
- [ ] AC-10.3: `as_any()` enables downcasting
- [ ] AC-10.4: `text_content()` returns valid UTF-8

---

## 5. Testing Requirements

### 5.1 Unit Tests - User Message

```
test_user_message_new
test_user_message_with_images
test_user_message_text_content
test_user_message_category
test_user_message_renderer_basic
test_user_message_renderer_wrapping
test_user_message_transcript_excludes_images
```

### 5.2 Unit Tests - Agent Message

```
test_agent_message_new_empty
test_agent_message_append_text
test_agent_message_append_multiple
test_agent_message_finalize
test_agent_message_streaming_state
test_agent_message_thinking
test_agent_message_tool_use_indicator
test_agent_renderer_shimmer_tick
test_agent_renderer_markdown_rendering
test_agent_renderer_cost_display
```

### 5.3 Unit Tests - Tool Call

```
test_tool_call_new
test_tool_call_state_pending
test_tool_call_state_running
test_tool_call_complete_success
test_tool_call_complete_error
test_tool_call_toggle_collapsed
test_tool_renderer_pending_style
test_tool_renderer_running_style
test_tool_renderer_complete_style
test_tool_renderer_error_style
test_tool_renderer_collapsed
```

### 5.4 Unit Tests - Exec Command

```
test_exec_new
test_exec_set_output_short
test_exec_set_output_truncated
test_exec_complete_success
test_exec_complete_failure
test_exec_approval_pending
test_exec_approval_approved
test_exec_approval_rejected
test_exec_expand_output
test_exec_is_success_true
test_exec_is_success_false
test_exec_renderer_command_display
test_exec_renderer_output_display
test_exec_renderer_exit_code_display
```

### 5.5 Unit Tests - Patch

```
test_patch_new_create
test_patch_new_modify
test_patch_new_delete
test_patch_new_rename
test_patch_stats_calculation
test_patch_toggle_collapsed
test_patch_approval_flow
test_patch_renderer_diff_highlighting
test_patch_renderer_stats_display
test_patch_renderer_collapsed
```

### 5.6 Unit Tests - Session Info

```
test_session_builder_empty
test_session_builder_model
test_session_builder_working_dir
test_session_builder_context_file
test_session_builder_context_url
test_session_builder_custom
test_session_builder_chaining
test_session_renderer_key_value_pairs
```

### 5.7 Unit Tests - System Cells

```
test_info_data_new
test_warning_data_new
test_error_data_new
test_error_data_with_details
test_plan_update_new
test_plan_item_states
test_info_renderer_style
test_warning_renderer_style
test_error_renderer_style
test_plan_renderer_state_indicators
```

### 5.8 Unit Tests - Streaming

```
test_stream_controller_new
test_stream_controller_start
test_stream_controller_append
test_stream_controller_finalize
test_stream_controller_cancel
test_stream_controller_is_streaming
test_stream_controller_tick
test_stream_controller_only_one_active
```

### 5.9 Unit Tests - Registry

```
test_registry_register_type
test_registry_downcast_correct_type
test_registry_downcast_wrong_type
test_registry_downcast_mut
test_downcast_convenience_functions
```

### 5.10 Property-Based Tests

```
proptest_user_message_text_roundtrip
proptest_agent_append_preserves_content
proptest_exec_output_truncation_consistent
proptest_patch_stats_match_content
proptest_all_cells_send_sync
```

### 5.11 Integration Tests

```
test_cell_in_box_dyn_cell_data
test_cell_downcast_after_boxing
test_streaming_full_cycle
test_renderer_width_zero_no_panic
test_renderer_width_one
```

---

## 6. Error Handling

### 6.1 No Panics Policy

All cell operations are infallible. Invalid states are handled gracefully:

| Situation | Behavior |
|-----------|----------|
| Append to non-streaming cell | No-op |
| Finalize non-streaming cell | No-op |
| Cancel when not streaming | No-op |
| Downcast to wrong type | Return None |
| Render with width 0 | Return empty lines |
| Empty content | Render appropriate placeholder |

### 6.2 Invalid Data Handling

| Invalid Input | Handling |
|---------------|----------|
| Empty command string | Store as-is, render as empty |
| Invalid patch content | Store as-is, render as plain text |
| Missing exit code | Show "running" or "unknown" |
| Negative duration | Clamp to zero |

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tui-core` | workspace | CellData trait, CellCategory |
| `tui-render` | workspace | Markdown rendering, word wrap |
| `ratatui` | 0.28+ | Line, Span, Style types |
| `chrono` | 0.4+ | DateTime for timestamps |
| `serde_json` | 1.0+ | Tool parameters storage |

### 7.2 Optional Dependencies

| Crate | Feature | Purpose |
|-------|---------|---------|
| `tokio` | - | Stream controller async support |

### 7.3 Forbidden Dependencies

- No terminal I/O
- No file I/O
- No network I/O
- No application logic

---

## 8. File Structure

```
tui-cells/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports
    ├── registry.rs             # CellRegistry, downcast helpers
    │
    ├── data/                   # CellData implementations
    │   ├── mod.rs
    │   ├── user.rs             # UserMessageData
    │   ├── agent.rs            # AgentMessageData, CostInfo
    │   ├── tool_call.rs        # ToolCallData, ToolState, ToolResult
    │   ├── exec.rs             # ExecCommandData, ExecOutput, ApprovalState
    │   ├── patch.rs            # PatchData, PatchType
    │   ├── session.rs          # SessionInfoData, SessionInfoBuilder
    │   ├── info.rs             # InfoData
    │   ├── warning.rs          # WarningData
    │   ├── error.rs            # ErrorData
    │   └── plan.rs             # PlanUpdateData, PlanItem
    │
    ├── render/                 # CellRenderer implementations
    │   ├── mod.rs
    │   ├── user.rs             # UserMessageRenderer
    │   ├── agent.rs            # AgentMessageRenderer, AgentStyles
    │   ├── tool_call.rs        # ToolCallRenderer
    │   ├── exec.rs             # ExecCommandRenderer
    │   ├── patch.rs            # PatchRenderer
    │   ├── session.rs          # SessionInfoRenderer
    │   ├── info.rs             # InfoRenderer
    │   ├── warning.rs          # WarningRenderer
    │   ├── error.rs            # ErrorRenderer
    │   └── plan.rs             # PlanUpdateRenderer
    │
    └── streaming/
        ├── mod.rs
        └── controller.rs       # StreamController
```

---

## 9. Thread Safety Requirements

| Type | Send | Sync | Notes |
|------|------|------|-------|
| All `*Data` types | Yes | Yes | Required by CellData trait |
| All `*Renderer` types | Yes | No | Contain mutable state (shimmer) |
| `StreamController` | No | No | Mutates cell references |
| `CellRegistry` | Yes | Yes | Immutable after construction |

---

## 10. Performance Requirements

- PR-1: `append_text()` MUST be O(1) amortized (string append)
- PR-2: Renderer MUST NOT re-parse markdown on every call (cache or single-pass)
- PR-3: Downcast MUST be O(1) (TypeId comparison)
- PR-4: Large outputs (>10MB) MUST use truncation, not full storage

---

## 11. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
