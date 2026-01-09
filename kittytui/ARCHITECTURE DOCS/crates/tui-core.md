# tui-core Technical Specification

## Overview

**Crate Name**: `tui-core`
**Purpose**: Define traits, primitives, and abstractions for the TUI framework
**Dependencies**: Minimal (ratatui, crossterm types only)
**Dependents**: All other tui-* crates

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: Cell Data Abstraction
- FR-1.1: Define a trait for conversation history cell data
- FR-1.2: Cell data MUST be separable from rendering logic
- FR-1.3: Cell data MUST support categorization (user message, agent message, tool call, error, etc.)
- FR-1.4: Cell data MUST provide plain text extraction for export/search
- FR-1.5: Cell data MUST support downcasting to concrete types
- FR-1.6: Cell data MUST be `Send + Sync` for async contexts

#### FR-2: Rendering Abstraction
- FR-2.1: Define a trait for rendering components to terminal buffers
- FR-2.2: Renderables MUST report their desired height for a given width
- FR-2.3: Define a separate trait for rendering cell data to lines
- FR-2.4: Cell renderers MUST support both display and transcript output modes

#### FR-3: Input Handling Abstraction
- FR-3.1: Define a trait for components that handle keyboard input
- FR-3.2: Input results MUST distinguish: consumed, submit, action request, ignored
- FR-3.3: Define a trait for modal/overlay views with completion state
- FR-3.4: Modals MUST report when they are complete and provide results

#### FR-4: Protocol Abstraction
- FR-4.1: Define traits for backend protocol events (incoming)
- FR-4.2: Define traits for backend protocol operations (outgoing)
- FR-4.3: Protocol types MUST be generic (not tied to any specific backend)
- FR-4.4: Define action types for TUI responses to protocol events

#### FR-5: Utility Types
- FR-5.1: Provide color manipulation utilities (blend, luminance detection)
- FR-5.2: Provide text utilities (display width, truncation)
- FR-5.3: Provide geometry utilities (insets, scroll state)
- FR-5.4: Provide TUI-specific error types

### 1.2 Non-Functional Requirements

- NFR-1: Zero business logic - traits and types only
- NFR-2: Minimal dependencies - only ratatui and crossterm for types
- NFR-3: No async runtime dependency
- NFR-4: All types must be `'static` compatible
- NFR-5: Documentation for all public items

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Actual implementations of traits (belong in other crates)
- NR-2: Terminal I/O operations
- NR-3: Rendering logic
- NR-4: State management
- NR-5: Any protocol-specific types

---

## 3. Interface Specification

### 3.1 Cell Data Trait

```rust
pub trait CellData: Send + Sync + Debug + Any {
    fn category(&self) -> CellCategory;
    fn text_content(&self) -> Cow<'_, str>;
    fn is_continuation(&self) -> bool;
    fn is_active(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

**CellCategory enum variants**:
| Variant | Description |
|---------|-------------|
| `UserMessage` | User-authored input |
| `AgentMessage` | Assistant/agent response |
| `ToolCall` | Tool/function invocation |
| `CommandOutput` | Shell command output |
| `SystemInfo` | Session info, configuration |
| `Warning` | Warning message |
| `Error` | Error message |
| `Info` | Informational message |

### 3.2 Renderable Trait

```rust
pub trait Renderable {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn desired_height(&self, width: u16) -> u16;
}
```

### 3.3 Cell Renderer Trait

```rust
pub trait CellRenderer<C: CellData> {
    fn render_lines(&self, cell: &C, width: u16) -> Vec<Line<'static>>;
    fn transcript_lines(&self, cell: &C, width: u16) -> Vec<Line<'static>>;
    fn height(&self, cell: &C, width: u16) -> u16;
}
```

### 3.4 Input Handler Trait

```rust
pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
    fn handle_paste(&mut self, text: String) -> InputResult;
    fn wants_focus(&self) -> bool;
}
```

**InputResult enum variants**:
| Variant | Description |
|---------|-------------|
| `Consumed` | Event handled, no further action |
| `Submit(String)` | Content submitted |
| `Action(InputAction)` | Action requested |
| `Ignored` | Event not handled, propagate |

**InputAction enum variants**:
| Variant | Description |
|---------|-------------|
| `ShowCommandMenu` | Show slash command picker |
| `ShowFilePicker` | Show file picker |
| `Cancel` | Cancel current operation |
| `Exit` | Exit application |
| `Custom(String)` | Application-specific action |

### 3.5 Modal View Trait

```rust
pub trait ModalView: InputHandler + Renderable {
    fn is_complete(&self) -> bool;
    fn result(&self) -> Option<ModalResult>;
}
```

### 3.6 Protocol Traits

```rust
pub trait ProtocolEvent: Debug + Send + 'static {
    fn category(&self) -> EventCategory;
}

pub trait ProtocolOp: Debug + Send + 'static {
    fn category(&self) -> OpCategory;
}

pub trait ProtocolHandler<E: ProtocolEvent> {
    fn handle(&mut self, event: E) -> Vec<ProtocolAction>;
}
```

### 3.7 Utility Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `is_light` | `(Rgb) -> bool` | Returns true if color is perceptually light |
| `blend` | `(Rgb, Rgb, f32) -> Rgb` | Alpha-blend two colors |
| `perceptual_distance` | `(Rgb, Rgb) -> f32` | CIE76 color distance |
| `display_width` | `(&str) -> usize` | Unicode-aware display width |
| `truncate` | `(&str, usize) -> String` | Truncate with ellipsis |
| `truncate_middle` | `(&str, usize) -> String` | Truncate from middle |

### 3.8 Geometry Types

**Insets**:
```rust
pub struct Insets {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}
```

**ScrollState**:
```rust
pub struct ScrollState {
    pub offset: usize,
    pub content_height: usize,
    pub viewport_height: usize,
}
```

---

## 4. Acceptance Criteria

### AC-1: Cell Data Trait
- [ ] AC-1.1: Any type implementing `CellData` can be stored in `Box<dyn CellData>`
- [ ] AC-1.2: `CellData` objects can be sent across threads
- [ ] AC-1.3: `CellData` can be downcast to concrete type using `as_any()`
- [ ] AC-1.4: `text_content()` returns valid UTF-8 for all cell types
- [ ] AC-1.5: `category()` returns appropriate variant for cell semantics

### AC-2: Renderable Trait
- [ ] AC-2.1: `desired_height()` returns value ≥ 1 for non-empty content
- [ ] AC-2.2: `desired_height()` returns 0 only for truly empty renderables
- [ ] AC-2.3: `render()` does not panic for any valid `Rect`
- [ ] AC-2.4: `render()` clips content to provided area (no buffer overflow)

### AC-3: Cell Renderer Trait
- [ ] AC-3.1: `render_lines()` returns `Vec<Line<'static>>` (owned, no lifetime issues)
- [ ] AC-3.2: `transcript_lines()` excludes UI-only elements (spinners, hints)
- [ ] AC-3.3: `height()` matches `render_lines().len()` after wrapping

### AC-4: Input Handler Trait
- [ ] AC-4.1: `handle_key()` returns `Ignored` for unhandled keys
- [ ] AC-4.2: `handle_key()` returns `Consumed` for handled keys that need no further action
- [ ] AC-4.3: `handle_paste()` defaults to `Ignored` if not overridden
- [ ] AC-4.4: `wants_focus()` defaults to `true` if not overridden

### AC-5: Modal View Trait
- [ ] AC-5.1: `is_complete()` returns `false` until modal should close
- [ ] AC-5.2: `result()` returns `Some` when `is_complete()` is `true`
- [ ] AC-5.3: Modal can be dismissed via `Cancel` action

### AC-6: Protocol Traits
- [ ] AC-6.1: `ProtocolEvent` implementors are `Send + 'static`
- [ ] AC-6.2: `ProtocolOp` implementors are `Send + 'static`
- [ ] AC-6.3: `ProtocolHandler::handle()` returns empty vec for no-op events

### AC-7: Utilities
- [ ] AC-7.1: `is_light()` returns `true` for `(255, 255, 255)`, `false` for `(0, 0, 0)`
- [ ] AC-7.2: `blend()` with alpha=0.0 returns bottom color
- [ ] AC-7.3: `blend()` with alpha=1.0 returns top color
- [ ] AC-7.4: `display_width()` handles emoji (width 2), CJK (width 2), ASCII (width 1)
- [ ] AC-7.5: `truncate()` adds ellipsis when truncating
- [ ] AC-7.6: `truncate()` returns original string if fits
- [ ] AC-7.7: `truncate_middle()` preserves start and end of string

### AC-8: Geometry
- [ ] AC-8.1: `Insets::horizontal()` returns `left + right`
- [ ] AC-8.2: `Insets::vertical()` returns `top + bottom`
- [ ] AC-8.3: `RectExt::inset()` uses saturating subtraction (no underflow)
- [ ] AC-8.4: `ScrollState::scroll_down()` clamps to valid range
- [ ] AC-8.5: `ScrollState::scroll_up()` does not underflow
- [ ] AC-8.6: `ScrollState::is_at_bottom()` is accurate

---

## 5. Testing Requirements

### 5.1 Unit Tests

#### Cell Data Tests
```
test_cell_data_send_sync
  - Verify CellData: Send + Sync bounds compile

test_cell_data_downcast
  - Create concrete CellData impl
  - Box as dyn CellData
  - Downcast back to concrete type
  - Verify data integrity

test_cell_category_all_variants
  - Each CellCategory variant is distinct
  - Debug formatting works for all variants
```

#### Renderable Tests
```
test_renderable_object_safety
  - Verify Renderable is object-safe (can use dyn Renderable)

test_renderable_zero_area
  - render() with 0-width area does not panic
  - render() with 0-height area does not panic
```

#### Input Handler Tests
```
test_input_result_variants
  - All InputResult variants constructible
  - Pattern matching works on all variants

test_input_action_custom
  - InputAction::Custom can hold arbitrary strings
  - Custom actions are equatable
```

#### Color Utility Tests
```
test_is_light_white -> true
test_is_light_black -> false
test_is_light_gray_128 -> boundary case
test_blend_alpha_0 -> returns bottom
test_blend_alpha_1 -> returns top
test_blend_alpha_0_5 -> returns midpoint
test_perceptual_distance_same_color -> 0.0
test_perceptual_distance_black_white -> large value
```

#### Text Utility Tests
```
test_display_width_ascii -> 1 per char
test_display_width_emoji -> 2 per emoji
test_display_width_cjk -> 2 per CJK char
test_display_width_combining -> handles combining marks
test_truncate_fits -> no change
test_truncate_exact -> no change
test_truncate_overflow -> adds ellipsis
test_truncate_empty -> returns empty
test_truncate_middle_short -> no change
test_truncate_middle_long -> preserves ends
```

#### Geometry Tests
```
test_insets_new
test_insets_uniform
test_insets_symmetric
test_insets_horizontal_vertical
test_rect_inset_normal
test_rect_inset_saturating -> no underflow
test_scroll_state_new
test_scroll_state_scroll_down_normal
test_scroll_state_scroll_down_clamped
test_scroll_state_scroll_up_normal
test_scroll_state_scroll_up_floor
test_scroll_state_is_at_bottom
```

### 5.2 Property-Based Tests

```
proptest_blend_alpha_bounds
  - For any colors and alpha in [0, 1], result channels are in [0, 255]

proptest_truncate_never_exceeds
  - For any string and width, result display_width <= width

proptest_inset_never_negative
  - For any Rect and Insets, result dimensions >= 0
```

### 5.3 Compile-Time Tests

```
compile_test_cell_data_send
  - fn assert_send<T: Send>() {}
  - assert_send::<Box<dyn CellData>>();

compile_test_cell_data_sync
  - fn assert_sync<T: Sync>() {}
  - assert_sync::<Box<dyn CellData>>();

compile_test_protocol_event_bounds
  - Verify ProtocolEvent: Debug + Send + 'static
```

---

## 6. Error Handling

### 6.1 TuiError Type

| Variant | Cause | Recovery |
|---------|-------|----------|
| `Terminal(io::Error)` | Terminal I/O failure | Restore terminal, exit |
| `Render(String)` | Rendering logic error | Log, continue |
| `Protocol(String)` | Backend communication error | Show error cell, continue |
| `Config(String)` | Configuration error | Show error, use defaults |

### 6.2 Error Conversion

- `From<io::Error>` -> `TuiError::Terminal`
- All other errors require explicit conversion

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.28+ | `Rect`, `Buffer`, `Line`, `Span`, `Style` types |
| `crossterm` | 0.28+ | `KeyEvent` type |
| `unicode-width` | 0.1+ | Display width calculation |

### 7.2 Forbidden Dependencies

- No async runtime (tokio, async-std)
- No logging/tracing
- No file I/O
- No network I/O
- No serialization (serde)

---

## 8. File Structure

```
tui-core/
├── Cargo.toml
└── src/
    ├── lib.rs              # Re-exports
    ├── traits/
    │   ├── mod.rs
    │   ├── cell.rs         # CellData, CellCategory
    │   ├── renderable.rs   # Renderable, CellRenderer, Insets
    │   ├── input.rs        # InputHandler, ModalView, InputResult
    │   └── protocol.rs     # ProtocolEvent, ProtocolOp, ProtocolHandler
    ├── types/
    │   ├── mod.rs
    │   ├── event.rs        # TuiEvent (re-export or wrapper)
    │   ├── geometry.rs     # ScrollState
    │   └── error.rs        # TuiError, TuiResult
    └── util/
        ├── mod.rs
        ├── color.rs        # RGB utilities
        └── text.rs         # String utilities
```

---

## 9. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
