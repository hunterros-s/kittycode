# Feature Roadmap

Step-by-step guide from MVP to full spec. Each feature is self-contained and testable.

**Spec files location:** `/Users/hunterross/Developer/codex/codex-rs/tui-docs/crates/`

---

## Phase 1: Core UX (Usability)

### 1.1 Auto-scroll to Bottom
**Problem:** New messages disappear off screen
**Crates:** tui-app
**Spec:** None (bug fix)
**Effort:** 30 min

When history exceeds viewport, always show most recent messages.

---

### 1.2 Word Wrapping
**Problem:** Long lines break layout
**Crates:** tui-render, tui-cells
**Spec:** `tui-render.md` sections:
- FR-2: Word Wrapping (FR-2.1 through FR-2.6)
- Interface 3.2: Word Wrap API
- AC-2: Word Wrapping acceptance criteria

**Effort:** 2-3 hours

Implement:
```rust
// tui-render
pub fn word_wrap(line: Line<'static>, width: usize, opts: WrapOptions) -> Vec<Line<'static>>;
```

Then update cell renderers to use it.

---

### 1.3 Scroll Navigation
**Problem:** Can't read old messages
**Crates:** tui-core, tui-app
**Spec:** `tui-core.md` section:
- Interface 3.8: ScrollState struct

**Effort:** 1-2 hours

Add PageUp/PageDown/Home/End to navigate history. Implement `ScrollState`:
```rust
pub struct ScrollState {
    pub offset: usize,
    pub content_height: usize,
    pub viewport_height: usize,
}
```

---

### 1.4 Inline Mode
**Problem:** Alternate screen doesn't match Claude Code UX
**Crates:** tui-terminal
**Spec:** `tui-terminal.md` sections:
- FR-5: Viewport Management (FR-5.1 through FR-5.5)
- Interface 3.2: Tui Coordinator API (`enter_alt_screen`, `leave_alt_screen`, `is_alt_screen`)
- AC-5: Viewport acceptance criteria

**Effort:** 3-4 hours

Render at bottom of terminal, preserve scrollback.

---

## Phase 2: Streaming

### 2.1 Streaming Controller
**Problem:** Responses appear all at once
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- FR-8: Streaming Support (FR-8.1 through FR-8.6)
- Interface 3.8: StreamController
- AC-8: Streaming acceptance criteria

**Effort:** 2-3 hours

Implement:
```rust
pub struct StreamController {
    active_cell: Option<CellId>,
    shimmer_tick: u8,
}

impl StreamController {
    pub fn start_stream(&mut self, cell_id: CellId);
    pub fn append_text(...);
    pub fn finalize(...);
}
```

---

### 2.2 OpenAI Streaming Integration
**Problem:** Need to receive chunks from API
**Crates:** tui-app
**Spec:** `tui-app.md` section:
- FR-5: Protocol Bridge (FR-5.5)

**Effort:** 2-3 hours

Use `async-openai` streaming:
```rust
let stream = client.chat().create_stream(request).await?;
while let Some(chunk) = stream.next().await { ... }
```

---

### 2.3 Shimmer Animation
**Problem:** No visual feedback during streaming
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- FR-2.6: Render shimmer effect during active streaming
- AC-2.6: Shimmer animates during streaming

**Effort:** 1-2 hours

Animate the streaming indicator with frame ticks.

---

## Phase 3: More Cell Types

### 3.1 Error Cells
**Problem:** Errors not displayed properly
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- Interface 3.7: System Cell Types (ErrorData)
- AC-7.3: Error cells render with error style
- AC-7.4: Error details expandable

**Effort:** 1 hour

---

### 3.2 Info/Warning Cells
**Problem:** No system messages
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- Interface 3.7: System Cell Types (InfoData, WarningData)
- AC-7.1, AC-7.2

**Effort:** 1 hour

---

### 3.3 Tool Call Cells
**Problem:** Can't see what tools agent is using
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- FR-3: Tool Call Cells (FR-3.1 through FR-3.5)
- Interface 3.3: Tool Call Types
- AC-3: Tool Call Cells acceptance criteria

**Effort:** 2-3 hours

---

### 3.4 Exec Command Cells
**Problem:** Can't display command execution
**Crates:** tui-cells
**Spec:** `tui-cells.md` sections:
- FR-4: Exec Command Cells (FR-4.1 through FR-4.7)
- Interface 3.4: Exec Command Types
- AC-4: Exec Command Cells acceptance criteria

**Effort:** 3-4 hours

---

## Phase 4: Input Enhancements

### 4.1 Input History
**Problem:** Can't recall previous messages
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-5: Input History (FR-5.1 through FR-5.6)
- Interface 3.3: Input History API
- AC-5: Input History acceptance criteria

**Effort:** 1-2 hours

Up/Down arrows cycle through previous inputs.

---

### 4.2 Multi-line Input
**Problem:** Can't write multi-line messages
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-4.4: Support configurable height
- AC-4.1: Enter in multi-line mode inserts newline
- AC-4.2: Enter in single-line mode returns Submit

**Effort:** 1-2 hours

Shift+Enter for newline, Enter to submit.

---

### 4.3 Composer Widget
**Problem:** Textarea lacks chat-specific features
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-6: Composer Widget (FR-6.1 through FR-6.7)
- Interface 3.4: Composer Widget API
- AC-6: Composer acceptance criteria

**Effort:** 2-3 hours

Wraps Textarea + History + attachment tracking.

---

### 4.4 Slash Commands
**Problem:** No /help, /clear, etc.
**Crates:** tui-input, tui-app
**Spec:** `tui-input.md` sections:
- FR-10: Command Popup (FR-10.1 through FR-10.5)
- Interface 3.8: Command Popup API

**Effort:** 2-3 hours

---

## Phase 5: Modals & Approvals

### 5.1 List Selection Modal
**Problem:** Need picker UI for commands, files
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-7: List Selection Modal (FR-7.1 through FR-7.8)
- Interface 3.5: List Selection Modal API
- AC-7: List Selection Modal acceptance criteria

**Effort:** 2-3 hours

---

### 5.2 Approval Modal
**Problem:** Can't approve/reject tool actions
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-8: Approval Modal (FR-8.1 through FR-8.7)
- Interface 3.6: Approval Modal API
- AC-8: Approval Modal acceptance criteria

**Effort:** 2-3 hours

---

### 5.3 Approval Flow Integration
**Problem:** Need end-to-end approval workflow
**Crates:** tui-app
**Spec:** `tui-app.md` sections:
- FR-7: Approval Workflow (FR-7.1 through FR-7.5)
- Interface 3.7: Approval System
- AC-7: Approvals acceptance criteria

**Effort:** 3-4 hours

---

## Phase 6: Terminal Features

### 6.1 Frame Rate Limiting
**Problem:** Excessive redraws
**Crates:** tui-terminal
**Spec:** `tui-terminal.md` sections:
- FR-4: Frame Scheduling (FR-4.2)
- Interface 3.4: FrameLimiter
- AC-4.4, AC-4.5

**Effort:** 1 hour

---

### 6.2 External Program Support
**Problem:** Can't launch editor, pager
**Crates:** tui-terminal, tui-app
**Spec:** `tui-terminal.md` sections:
- FR-7: External Program Support (FR-7.1 through FR-7.4)
- Interface 3.2: `with_restored()` method
- AC-6: External Programs acceptance criteria

`tui-app.md` sections:
- FR-8: External Program Execution
- Interface 3.8: External Program Support

**Effort:** 2-3 hours

---

### 6.3 Notifications
**Problem:** No alerts when unfocused
**Crates:** tui-terminal
**Spec:** `tui-terminal.md` sections:
- FR-6.3: Support desktop notifications (OSC 9)
- Interface 3.6: Notification API
- AC-7: Notifications acceptance criteria

**Effort:** 1-2 hours

---

## Phase 7: Polish

### 7.1 Syntax Highlighting
**Problem:** Code blocks are plain text
**Crates:** tui-render
**Spec:** `tui-render.md` sections:
- FR-3: Syntax Highlighting (FR-3.1 through FR-3.4)
- Interface 3.3: Syntax Highlighting API
- AC-3: Syntax Highlighting acceptance criteria

**Effort:** 2-3 hours

---

### 7.2 Diff Rendering
**Problem:** Can't display file changes nicely
**Crates:** tui-render
**Spec:** `tui-render.md` sections:
- FR-4: Diff Rendering (FR-4.1 through FR-4.4)
- Interface 3.4: Diff Rendering API
- AC-4: Diff Rendering acceptance criteria

**Effort:** 2-3 hours

---

### 7.3 Session Persistence
**Problem:** History lost on exit
**Crates:** tui-app
**Spec:** `tui-app.md` sections:
- FR-10: Session Management (FR-10.1 through FR-10.5)
- Interface 3.10: Session Management
- AC-9: Onboarding (session picker parts)

**Effort:** 3-4 hours

---

### 7.4 Status Indicators
**Problem:** No spinners, progress bars
**Crates:** tui-input
**Spec:** `tui-input.md` sections:
- FR-12: Status Indicators (FR-12.1 through FR-12.5)
- Interface 3.10: Status Indicator API
- AC-9: Status Indicators acceptance criteria

**Effort:** 1-2 hours

---

## Quick Reference

Copy this to Claude Code when implementing a feature:

```
Implementing [FEATURE NAME] for kittytui.

Reference specs:
- /Users/hunterross/Developer/codex/codex-rs/tui-docs/crates/[CRATE].md
  - Sections: [LIST FROM ABOVE]

Current codebase: /Users/hunterross/Developer/codex/kittytui/
```

---

## Estimated Total Effort

| Phase | Features | Effort |
|-------|----------|--------|
| Phase 1: Core UX | 4 | 7-10 hours |
| Phase 2: Streaming | 3 | 5-8 hours |
| Phase 3: Cell Types | 4 | 7-9 hours |
| Phase 4: Input | 4 | 6-10 hours |
| Phase 5: Modals | 3 | 7-10 hours |
| Phase 6: Terminal | 3 | 4-6 hours |
| Phase 7: Polish | 4 | 8-12 hours |
| **Total** | **25** | **44-65 hours** |
