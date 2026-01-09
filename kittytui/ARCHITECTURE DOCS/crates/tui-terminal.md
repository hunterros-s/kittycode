# tui-terminal Technical Specification

## Overview

**Crate Name**: `tui-terminal`
**Purpose**: Terminal lifecycle, event streaming, frame scheduling, viewport management
**Dependencies**: `tui-core`, `ratatui`, `crossterm`, `tokio`
**Dependents**: `tui-app`

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: Terminal Initialization
- FR-1.1: Verify stdin/stdout are TTYs before initialization
- FR-1.2: Enable raw mode for character-by-character input
- FR-1.3: Enable bracketed paste mode
- FR-1.4: Enable focus change event reporting
- FR-1.5: Enable keyboard enhancement flags (if supported)
- FR-1.6: Install panic hook to restore terminal on crash
- FR-1.7: Provide detection of keyboard enhancement support

#### FR-2: Terminal Restoration
- FR-2.1: Disable raw mode
- FR-2.2: Pop keyboard enhancement flags
- FR-2.3: Disable bracketed paste
- FR-2.4: Show cursor
- FR-2.5: Leave alternate screen (if active)
- FR-2.6: Provide partial restoration (keep raw mode for external programs)

#### FR-3: Event Stream
- FR-3.1: Provide unified stream of TuiEvent (Key, Paste, Draw, Resize, Focus)
- FR-3.2: Support pausing event stream (for external program execution)
- FR-3.3: Support resuming event stream
- FR-3.4: Track terminal focus state
- FR-3.5: Coalesce multiple draw requests

#### FR-4: Frame Scheduling
- FR-4.1: Provide clonable handle for requesting redraws
- FR-4.2: Support frame rate limiting
- FR-4.3: Frame requests MUST be non-blocking
- FR-4.4: Frame requests from any thread MUST work

#### FR-5: Viewport Management
- FR-5.1: Support inline mode (draw at bottom of terminal)
- FR-5.2: Support alternate screen mode (full screen)
- FR-5.3: Save/restore viewport when switching modes
- FR-5.4: Support history line insertion (inline mode)
- FR-5.5: Calculate correct drawing area for given height

#### FR-6: Platform Support
- FR-6.1: Flush stdin buffer after external program (Unix: tcflush, Windows: FlushConsoleInputBuffer)
- FR-6.2: Support Unix job control (Ctrl-Z suspend/resume)
- FR-6.3: Support desktop notifications (OSC 9)

#### FR-7: External Program Support
- FR-7.1: Provide async wrapper for running external programs with restored terminal
- FR-7.2: Pause events before external program
- FR-7.3: Flush input after external program
- FR-7.4: Resume events after external program

### 1.2 Non-Functional Requirements

- NFR-1: Panic hook MUST restore terminal before unwinding
- NFR-2: Drop MUST restore terminal as fallback
- NFR-3: All public types MUST be Send where applicable
- NFR-4: Frame requester MUST be Clone + Send + Sync
- NFR-5: Event stream MUST be cancel-safe

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Widget rendering (belongs in tui-cells/tui-input)
- NR-2: Application state management
- NR-3: Protocol handling
- NR-4: Mouse event handling (keyboard only)
- NR-5: Multiple terminal support

---

## 3. Interface Specification

### 3.1 Initialization API

```rust
/// Initialize terminal for TUI operation
pub fn init() -> io::Result<Terminal>;

/// Restore terminal to normal state
pub fn restore() -> io::Result<()>;

/// Restore but keep raw mode (for external programs needing raw input)
pub fn restore_keep_raw() -> io::Result<()>;

/// Check if keyboard enhancement is supported
pub fn keyboard_enhancement_supported() -> bool;

/// Type alias for terminal backend
pub type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
```

### 3.2 Tui Coordinator API

```rust
pub struct Tui {
    // Private fields
}

impl Tui {
    /// Create coordinator from initialized terminal
    pub fn new(terminal: Terminal) -> Self;

    /// Get handle for requesting frame redraws
    pub fn frame_requester(&self) -> FrameRequester;

    /// Check if keyboard enhancement is enabled
    pub fn keyboard_enhanced(&self) -> bool;

    /// Check if alternate screen is active
    pub fn is_alt_screen(&self) -> bool;

    /// Get terminal size
    pub fn size(&self) -> io::Result<Rect>;

    /// Get event stream
    pub fn event_stream(&self) -> EventStream;

    /// Pause event processing
    pub fn pause_events(&mut self);

    /// Resume event processing
    pub fn resume_events(&mut self);

    /// Enter alternate screen mode
    pub fn enter_alt_screen(&mut self) -> io::Result<()>;

    /// Leave alternate screen mode
    pub fn leave_alt_screen(&mut self) -> io::Result<()>;

    /// Draw a frame
    pub fn draw<F>(&mut self, height: u16, draw_fn: F) -> io::Result<()>
    where
        F: FnOnce(Rect, &mut Buffer);

    /// Insert lines into scrollback (inline mode)
    pub fn insert_history(&mut self, lines: Vec<Line<'static>>);

    /// Send desktop notification (only if unfocused)
    pub fn notify(&self, message: &str) -> bool;

    /// Run function with terminal temporarily restored
    pub async fn with_restored<F, Fut, R>(
        &mut self,
        keep_raw: bool,
        f: F
    ) -> io::Result<R>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = R>;
}
```

### 3.3 Event Types

```rust
pub enum TuiEvent {
    Key(KeyEvent),
    Paste(String),
    Draw,
    Resize { width: u16, height: u16 },
    FocusChanged(bool),
}

pub struct EventStream { /* ... */ }

impl Stream for EventStream {
    type Item = TuiEvent;
}
```

### 3.4 Frame Scheduling API

```rust
#[derive(Clone)]
pub struct FrameRequester { /* ... */ }

impl FrameRequester {
    /// Request a frame redraw (non-blocking, coalesced)
    pub fn request(&self);
}

pub struct FrameLimiter { /* ... */ }

impl FrameLimiter {
    pub fn new(fps: u32) -> Self;
    pub fn should_draw(&mut self) -> bool;
    pub fn time_until_next(&self) -> Duration;
}
```

### 3.5 Platform API

```rust
/// Flush pending stdin input
pub fn flush_input();

/// Unix job control context (Unix only)
#[cfg(unix)]
pub struct SuspendContext {
    pub fn new(cursor_y: u16) -> Self;
    pub fn cursor_y(&self) -> u16;
}
```

### 3.6 Notification API

```rust
pub enum NotificationBackend {
    Osc9,
    None,
}

impl NotificationBackend {
    pub fn detect() -> Option<Self>;
    pub fn notify(&self, message: &str) -> bool;
}
```

---

## 4. Acceptance Criteria

### AC-1: Initialization
- [ ] AC-1.1: `init()` fails with error if stdin is not a TTY
- [ ] AC-1.2: `init()` fails with error if stdout is not a TTY
- [ ] AC-1.3: `init()` enables raw mode (verified by crossterm state)
- [ ] AC-1.4: `init()` enables bracketed paste
- [ ] AC-1.5: Panic after `init()` restores terminal
- [ ] AC-1.6: `keyboard_enhancement_supported()` returns consistent value

### AC-2: Restoration
- [ ] AC-2.1: `restore()` disables raw mode
- [ ] AC-2.2: `restore()` shows cursor
- [ ] AC-2.3: `restore()` leaves alternate screen
- [ ] AC-2.4: `restore_keep_raw()` keeps raw mode enabled
- [ ] AC-2.5: Multiple `restore()` calls are safe (idempotent)

### AC-3: Event Stream
- [ ] AC-3.1: Key events produce `TuiEvent::Key`
- [ ] AC-3.2: Bracketed paste produces `TuiEvent::Paste`
- [ ] AC-3.3: `frame_requester().request()` produces `TuiEvent::Draw`
- [ ] AC-3.4: Terminal resize produces `TuiEvent::Resize`
- [ ] AC-3.5: Focus change produces `TuiEvent::FocusChanged`
- [ ] AC-3.6: Paused stream produces no events
- [ ] AC-3.7: Resumed stream produces events again
- [ ] AC-3.8: Multiple rapid `request()` calls produce single `Draw`

### AC-4: Frame Scheduling
- [ ] AC-4.1: `FrameRequester` is `Clone + Send + Sync`
- [ ] AC-4.2: `request()` from background task works
- [ ] AC-4.3: `request()` never blocks
- [ ] AC-4.4: `FrameLimiter::should_draw()` respects FPS limit
- [ ] AC-4.5: `FrameLimiter::time_until_next()` is accurate

### AC-5: Viewport
- [ ] AC-5.1: `enter_alt_screen()` switches to alternate buffer
- [ ] AC-5.2: `leave_alt_screen()` returns to inline mode
- [ ] AC-5.3: `is_alt_screen()` reflects current state
- [ ] AC-5.4: `draw()` in inline mode draws at bottom
- [ ] AC-5.5: `draw()` in alt screen mode uses full area
- [ ] AC-5.6: `insert_history()` adds lines above input area

### AC-6: External Programs
- [ ] AC-6.1: `with_restored()` restores terminal before function
- [ ] AC-6.2: `with_restored()` re-initializes terminal after function
- [ ] AC-6.3: `with_restored()` flushes input after function
- [ ] AC-6.4: Events are paused during `with_restored()`
- [ ] AC-6.5: Events resume after `with_restored()`

### AC-7: Notifications
- [ ] AC-7.1: `notify()` returns `false` when focused
- [ ] AC-7.2: `notify()` returns `true` when unfocused and supported
- [ ] AC-7.3: `NotificationBackend::detect()` returns `Some` in supported terminals

### AC-8: Drop Safety
- [ ] AC-8.1: Dropping `Tui` restores terminal
- [ ] AC-8.2: Dropping `Tui` does not panic

---

## 5. Testing Requirements

### 5.1 Unit Tests

```
test_frame_requester_clone
test_frame_requester_send_sync
test_frame_limiter_new
test_frame_limiter_respects_fps
test_frame_limiter_time_until_next
test_tui_event_variants
test_notification_backend_detect
```

### 5.2 Integration Tests (require TTY)

```
test_init_restore_cycle
test_init_twice_fails
test_restore_idempotent
test_event_stream_key
test_event_stream_paste
test_event_stream_draw_request
test_event_stream_pause_resume
test_alt_screen_enter_leave
test_with_restored_external_program
```

### 5.3 Platform-Specific Tests

```
#[cfg(unix)]
test_flush_input_unix
test_suspend_context

#[cfg(windows)]
test_flush_input_windows
```

### 5.4 Stress Tests

```
test_rapid_frame_requests_coalesce
test_concurrent_frame_requests
test_event_stream_high_throughput
```

---

## 6. Error Handling

### 6.1 Error Types

| Function | Error Condition | Error Type |
|----------|-----------------|------------|
| `init()` | Not a TTY | `io::Error` (Other) |
| `init()` | Raw mode fails | `io::Error` |
| `restore()` | Disable raw mode fails | `io::Error` |
| `enter_alt_screen()` | Execute fails | `io::Error` |
| `leave_alt_screen()` | Execute fails | `io::Error` |
| `draw()` | Terminal draw fails | `io::Error` |
| `size()` | Query fails | `io::Error` |

### 6.2 Panic Behavior

- Panic hook restores terminal (best effort)
- No panics in normal operation
- `Drop` does not panic (logs errors instead)

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tui-core` | workspace | TuiEvent type |
| `ratatui` | 0.28+ | Terminal, Backend, Buffer |
| `crossterm` | 0.28+ | Raw mode, events, alternate screen |
| `tokio` | 1.0+ | broadcast channel, async |
| `tokio-stream` | 0.1+ | Stream trait |
| `futures` | 0.3+ | StreamExt |

### 7.2 Platform-Specific Dependencies

| Crate | Platform | Purpose |
|-------|----------|---------|
| `libc` | Unix | tcflush |
| `windows-sys` | Windows | FlushConsoleInputBuffer |

### 7.3 Forbidden Dependencies

- No rendering crates
- No application logic
- No protocol handling

---

## 8. File Structure

```
tui-terminal/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports
    ├── terminal.rs             # init(), restore(), Terminal type
    ├── tui.rs                  # Tui coordinator struct
    ├── events/
    │   ├── mod.rs
    │   ├── types.rs            # TuiEvent enum
    │   ├── stream.rs           # EventStream implementation
    │   └── broker.rs           # EventBroker (pause/resume)
    ├── frame/
    │   ├── mod.rs
    │   ├── requester.rs        # FrameRequester
    │   └── limiter.rs          # FrameLimiter
    ├── viewport/
    │   ├── mod.rs              # ViewportManager
    │   ├── inline.rs           # Inline mode logic
    │   └── alternate.rs        # Alternate screen logic
    ├── platform/
    │   ├── mod.rs              # flush_input()
    │   ├── unix.rs             # Unix-specific (tcflush, SuspendContext)
    │   └── windows.rs          # Windows-specific
    └── notify/
        ├── mod.rs
        ├── backend.rs          # NotificationBackend
        └── osc9.rs             # OSC 9 implementation
```

---

## 9. Thread Safety Requirements

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `Terminal` | No | No | Owned by Tui |
| `Tui` | No | No | Main thread only |
| `FrameRequester` | Yes | Yes | Cross-thread frame requests |
| `EventStream` | No | No | Single consumer |
| `FrameLimiter` | Yes | No | Per-thread timing |

---

## 10. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
