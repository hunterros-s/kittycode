# tui-input Technical Specification

## Overview

**Crate Name**: `tui-input`
**Purpose**: User input widgets, text editing, modals, and status indicators
**Dependencies**: `tui-core`, `tui-render`, `ratatui`, `crossterm`
**Dependents**: `tui-app`

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: Text Buffer
- FR-1.1: Store multiline text content
- FR-1.2: Track cursor position (row, column)
- FR-1.3: Support byte-index and grapheme-cluster positioning
- FR-1.4: Support text insertion at cursor
- FR-1.5: Support text deletion (backspace, delete, word delete)
- FR-1.6: Support line operations (new line, delete line, join lines)
- FR-1.7: Track modification state (dirty flag)

#### FR-2: Cursor Movement
- FR-2.1: Move left/right by character
- FR-2.2: Move left/right by word
- FR-2.3: Move up/down by line
- FR-2.4: Move to line start/end
- FR-2.5: Move to buffer start/end
- FR-2.6: Handle cursor at buffer boundaries gracefully
- FR-2.7: Support cursor position queries (at_start, at_end, at_line_start)

#### FR-3: Text Selection
- FR-3.1: Track selection anchor and cursor
- FR-3.2: Support shift+movement to extend selection
- FR-3.3: Support select-all
- FR-3.4: Support selection deletion
- FR-3.5: Support selection replacement (type over selection)
- FR-3.6: Provide selected text extraction

#### FR-4: Textarea Widget
- FR-4.1: Render text buffer with cursor
- FR-4.2: Handle keyboard input for editing
- FR-4.3: Handle paste events
- FR-4.4: Support configurable height (single-line vs multi-line)
- FR-4.5: Support scroll when content exceeds visible area
- FR-4.6: Support placeholder text when empty
- FR-4.7: Report desired height based on content

#### FR-5: Input History
- FR-5.1: Store previous inputs (LIFO)
- FR-5.2: Navigate history with up/down arrows
- FR-5.3: Preserve current input when navigating
- FR-5.4: Support history search (optional)
- FR-5.5: Configurable maximum history size
- FR-5.6: Return to current input from history

#### FR-6: Composer Widget
- FR-6.1: Combine textarea with history navigation
- FR-6.2: Support slash command detection (/command)
- FR-6.3: Support file attachment (@file syntax)
- FR-6.4: Support image attachments
- FR-6.5: Report submission (Enter) vs newline (Shift+Enter)
- FR-6.6: Track attached files/images
- FR-6.7: Support input clearing on submit

#### FR-7: List Selection Modal
- FR-7.1: Display list of selectable items
- FR-7.2: Support keyboard navigation (up/down/j/k)
- FR-7.3: Support selection confirmation (Enter)
- FR-7.4: Support cancellation (Escape)
- FR-7.5: Support filtering/search
- FR-7.6: Support scrolling for long lists
- FR-7.7: Highlight current selection
- FR-7.8: Support item descriptions

#### FR-8: Approval Modal
- FR-8.1: Display approval prompt
- FR-8.2: Show action to be approved
- FR-8.3: Support accept (y/Enter)
- FR-8.4: Support reject (n/Escape)
- FR-8.5: Support "always approve" option
- FR-8.6: Support "edit" option for commands
- FR-8.7: Display keyboard shortcuts

#### FR-9: Feedback Modal
- FR-9.1: Collect user feedback text
- FR-9.2: Support category selection
- FR-9.3: Support submission
- FR-9.4: Support cancellation
- FR-9.5: Validate non-empty feedback

#### FR-10: Command Popup
- FR-10.1: Display available slash commands
- FR-10.2: Filter commands by typed prefix
- FR-10.3: Show command descriptions
- FR-10.4: Support selection and insertion
- FR-10.5: Auto-dismiss on selection or escape

#### FR-11: File Picker Popup
- FR-11.1: Display file suggestions
- FR-11.2: Filter by typed path prefix
- FR-11.3: Support directory navigation
- FR-11.4: Support multiple selection
- FR-11.5: Show file type indicators

#### FR-12: Status Indicators
- FR-12.1: Animated spinner with configurable frames
- FR-12.2: Progress bar (determinate)
- FR-12.3: Progress indicator (indeterminate)
- FR-12.4: Status text display
- FR-12.5: Spinner tick animation

### 1.2 Non-Functional Requirements

- NFR-1: All widgets MUST implement InputHandler trait
- NFR-2: All modals MUST implement ModalView trait
- NFR-3: Cursor operations MUST be O(1) for single moves
- NFR-4: Text insertion MUST be O(n) worst case (n = text length)
- NFR-5: All widgets MUST be responsive (no blocking operations)
- NFR-6: Unicode support (emoji, CJK, combining characters)

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Syntax highlighting in textarea (use tui-render)
- NR-2: File system access (app layer provides file lists)
- NR-3: Network requests
- NR-4: Clipboard access (app layer handles)
- NR-5: Rich text editing (only plain text)
- NR-6: Undo/redo (optional, not required)

---

## 3. Interface Specification

### 3.1 Text Buffer API

```rust
pub struct TextBuffer {
    lines: Vec<String>,
    cursor: CursorPosition,
    selection: Option<Selection>,
    dirty: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

pub struct Selection {
    pub anchor: CursorPosition,
    pub cursor: CursorPosition,
}

impl TextBuffer {
    pub fn new() -> Self;
    pub fn from_str(s: &str) -> Self;

    // Content access
    pub fn content(&self) -> String;
    pub fn line(&self, row: usize) -> Option<&str>;
    pub fn line_count(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn is_dirty(&self) -> bool;
    pub fn clear_dirty(&mut self);

    // Cursor access
    pub fn cursor(&self) -> CursorPosition;
    pub fn set_cursor(&mut self, pos: CursorPosition);
    pub fn at_start(&self) -> bool;
    pub fn at_end(&self) -> bool;
    pub fn at_line_start(&self) -> bool;
    pub fn at_line_end(&self) -> bool;

    // Cursor movement
    pub fn move_left(&mut self);
    pub fn move_right(&mut self);
    pub fn move_up(&mut self);
    pub fn move_down(&mut self);
    pub fn move_word_left(&mut self);
    pub fn move_word_right(&mut self);
    pub fn move_to_line_start(&mut self);
    pub fn move_to_line_end(&mut self);
    pub fn move_to_start(&mut self);
    pub fn move_to_end(&mut self);

    // Editing
    pub fn insert_char(&mut self, c: char);
    pub fn insert_str(&mut self, s: &str);
    pub fn insert_newline(&mut self);
    pub fn delete_backward(&mut self);   // Backspace
    pub fn delete_forward(&mut self);    // Delete
    pub fn delete_word_backward(&mut self);
    pub fn delete_word_forward(&mut self);
    pub fn delete_line(&mut self);
    pub fn clear(&mut self);

    // Selection
    pub fn selection(&self) -> Option<&Selection>;
    pub fn select_all(&mut self);
    pub fn clear_selection(&mut self);
    pub fn selected_text(&self) -> Option<String>;
    pub fn delete_selection(&mut self) -> bool;
    pub fn extend_selection_left(&mut self);
    pub fn extend_selection_right(&mut self);
    pub fn extend_selection_up(&mut self);
    pub fn extend_selection_down(&mut self);
}
```

### 3.2 Textarea Widget API

```rust
pub struct Textarea {
    buffer: TextBuffer,
    scroll_offset: usize,
    placeholder: Option<String>,
    max_height: Option<u16>,
    single_line: bool,
}

impl Textarea {
    pub fn new() -> Self;
    pub fn single_line() -> Self;
    pub fn with_placeholder(placeholder: &str) -> Self;
    pub fn with_max_height(height: u16) -> Self;

    pub fn buffer(&self) -> &TextBuffer;
    pub fn buffer_mut(&mut self) -> &mut TextBuffer;
    pub fn set_content(&mut self, content: &str);
    pub fn content(&self) -> String;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);

    // For rendering
    pub fn visible_lines(&self, height: u16) -> impl Iterator<Item = &str>;
    pub fn cursor_screen_position(&self, width: u16) -> (u16, u16);
}

impl InputHandler for Textarea {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
    fn handle_paste(&mut self, text: String) -> InputResult;
    fn wants_focus(&self) -> bool;
}

impl Renderable for Textarea {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn desired_height(&self, width: u16) -> u16;
}
```

### 3.3 Input History API

```rust
pub struct InputHistory {
    entries: VecDeque<String>,
    max_size: usize,
    position: Option<usize>,
    current: String,
}

impl InputHistory {
    pub fn new(max_size: usize) -> Self;

    pub fn push(&mut self, input: String);
    pub fn previous(&mut self, current: &str) -> Option<&str>;
    pub fn next(&mut self) -> Option<&str>;
    pub fn current(&self) -> &str;
    pub fn reset_position(&mut self);
    pub fn is_navigating(&self) -> bool;
    pub fn len(&self) -> usize;
}
```

### 3.4 Composer Widget API

```rust
pub struct Composer {
    textarea: Textarea,
    history: InputHistory,
    attachments: Vec<Attachment>,
    pending_command: Option<String>,
}

pub enum Attachment {
    File(PathBuf),
    Image { path: PathBuf, data: Vec<u8> },
}

pub enum ComposerResult {
    Consumed,
    Submit { text: String, attachments: Vec<Attachment> },
    ShowCommandMenu,
    ShowFilePicker,
    Cancel,
    Ignored,
}

impl Composer {
    pub fn new(history_size: usize) -> Self;

    pub fn content(&self) -> &str;
    pub fn set_content(&mut self, content: &str);
    pub fn clear(&mut self);
    pub fn is_empty(&self) -> bool;

    pub fn add_attachment(&mut self, attachment: Attachment);
    pub fn clear_attachments(&mut self);
    pub fn attachments(&self) -> &[Attachment];

    pub fn push_history(&mut self, input: String);

    pub fn handle_key(&mut self, key: KeyEvent) -> ComposerResult;
    pub fn handle_paste(&mut self, text: String) -> ComposerResult;
}

impl Renderable for Composer {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn desired_height(&self, width: u16) -> u16;
}
```

### 3.5 List Selection Modal API

```rust
pub struct ListItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

pub struct ListSelectionModal {
    items: Vec<ListItem>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
    scroll_offset: usize,
    title: String,
}

pub enum ListSelectionResult {
    Selected(String),  // Item ID
    Cancelled,
    Pending,
}

impl ListSelectionModal {
    pub fn new(items: Vec<ListItem>, title: &str) -> Self;

    pub fn selected_item(&self) -> Option<&ListItem>;
    pub fn filter(&self) -> &str;
}

impl ModalView for ListSelectionModal {
    fn is_complete(&self) -> bool;
    fn result(&self) -> Option<ModalResult>;
}

impl InputHandler for ListSelectionModal {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
    fn handle_paste(&mut self, text: String) -> InputResult;
    fn wants_focus(&self) -> bool;
}

impl Renderable for ListSelectionModal {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn desired_height(&self, width: u16) -> u16;
}
```

### 3.6 Approval Modal API

```rust
pub struct ApprovalModal {
    title: String,
    description: String,
    action_preview: Option<String>,
    allow_always: bool,
    allow_edit: bool,
    result: Option<ApprovalResult>,
}

pub enum ApprovalResult {
    Approved,
    ApprovedAlways,
    Rejected,
    Edit,
}

impl ApprovalModal {
    pub fn new(title: &str, description: &str) -> Self;
    pub fn with_action_preview(self, preview: &str) -> Self;
    pub fn with_always_option(self) -> Self;
    pub fn with_edit_option(self) -> Self;
}

impl ModalView for ApprovalModal {
    fn is_complete(&self) -> bool;
    fn result(&self) -> Option<ModalResult>;
}

impl InputHandler for ApprovalModal { /* ... */ }
impl Renderable for ApprovalModal { /* ... */ }
```

### 3.7 Feedback Modal API

```rust
pub struct FeedbackModal {
    textarea: Textarea,
    category: FeedbackCategory,
    result: Option<FeedbackResult>,
}

pub enum FeedbackCategory {
    Bug,
    Feature,
    Other,
}

pub struct FeedbackResult {
    pub text: String,
    pub category: FeedbackCategory,
}

impl FeedbackModal {
    pub fn new() -> Self;
}

impl ModalView for FeedbackModal { /* ... */ }
impl InputHandler for FeedbackModal { /* ... */ }
impl Renderable for FeedbackModal { /* ... */ }
```

### 3.8 Command Popup API

```rust
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
}

pub struct CommandPopup {
    commands: Vec<CommandInfo>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
}

impl CommandPopup {
    pub fn new(commands: Vec<CommandInfo>) -> Self;
    pub fn set_filter(&mut self, filter: &str);
    pub fn selected_command(&self) -> Option<&CommandInfo>;
    pub fn move_up(&mut self);
    pub fn move_down(&mut self);
    pub fn confirm(&mut self) -> Option<String>;
}

impl Renderable for CommandPopup { /* ... */ }
```

### 3.9 File Picker Popup API

```rust
pub struct FileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub display_name: String,
}

pub struct FilePickerPopup {
    entries: Vec<FileEntry>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
    multi_select: bool,
    selected_indices: HashSet<usize>,
}

impl FilePickerPopup {
    pub fn new(entries: Vec<FileEntry>) -> Self;
    pub fn multi_select(self) -> Self;
    pub fn set_filter(&mut self, filter: &str);
    pub fn toggle_selection(&mut self);
    pub fn selected_paths(&self) -> Vec<PathBuf>;
}

impl Renderable for FilePickerPopup { /* ... */ }
```

### 3.10 Status Indicator API

```rust
pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    label: Option<String>,
}

impl Spinner {
    pub fn new() -> Self;
    pub fn dots() -> Self;
    pub fn braille() -> Self;
    pub fn with_label(self, label: &str) -> Self;
    pub fn tick(&mut self);
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}

pub struct ProgressBar {
    progress: f64,  // 0.0 to 1.0
    label: Option<String>,
    show_percentage: bool,
}

impl ProgressBar {
    pub fn new() -> Self;
    pub fn set_progress(&mut self, progress: f64);
    pub fn with_label(self, label: &str) -> Self;
    pub fn show_percentage(self) -> Self;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

---

## 4. Acceptance Criteria

### AC-1: Text Buffer
- [ ] AC-1.1: `TextBuffer::new()` creates empty buffer
- [ ] AC-1.2: `from_str()` correctly splits lines
- [ ] AC-1.3: `content()` rejoins lines with newlines
- [ ] AC-1.4: `insert_char()` inserts at cursor position
- [ ] AC-1.5: `insert_str()` handles multiline strings
- [ ] AC-1.6: `delete_backward()` removes character before cursor
- [ ] AC-1.7: `delete_backward()` joins lines at line start
- [ ] AC-1.8: Cursor cannot move outside buffer bounds
- [ ] AC-1.9: `is_dirty()` true after modification
- [ ] AC-1.10: `clear_dirty()` resets dirty flag

### AC-2: Cursor Movement
- [ ] AC-2.1: `move_left()` at position 0 is no-op
- [ ] AC-2.2: `move_right()` at end is no-op
- [ ] AC-2.3: `move_up()` at first line is no-op
- [ ] AC-2.4: `move_down()` at last line is no-op
- [ ] AC-2.5: `move_word_left()` stops at word boundaries
- [ ] AC-2.6: `move_word_right()` stops at word boundaries
- [ ] AC-2.7: `move_to_line_start()` moves to column 0
- [ ] AC-2.8: `move_to_line_end()` moves to line length

### AC-3: Selection
- [ ] AC-3.1: `select_all()` selects entire buffer
- [ ] AC-3.2: `selected_text()` returns selected content
- [ ] AC-3.3: `delete_selection()` removes selected text
- [ ] AC-3.4: `insert_char()` with selection replaces selection
- [ ] AC-3.5: Movement without shift clears selection

### AC-4: Textarea
- [ ] AC-4.1: Enter in multi-line mode inserts newline
- [ ] AC-4.2: Enter in single-line mode returns Submit
- [ ] AC-4.3: Placeholder shown when empty
- [ ] AC-4.4: Placeholder hidden when content present
- [ ] AC-4.5: Scroll adjusts to keep cursor visible
- [ ] AC-4.6: Paste inserts text at cursor
- [ ] AC-4.7: Ctrl+A selects all

### AC-5: Input History
- [ ] AC-5.1: `push()` adds to history
- [ ] AC-5.2: `push()` respects max_size
- [ ] AC-5.3: `previous()` returns previous entry
- [ ] AC-5.4: `previous()` preserves current input
- [ ] AC-5.5: `next()` returns next entry
- [ ] AC-5.6: `next()` at end returns current
- [ ] AC-5.7: `reset_position()` returns to current

### AC-6: Composer
- [ ] AC-6.1: Enter submits (returns Submit result)
- [ ] AC-6.2: Shift+Enter inserts newline
- [ ] AC-6.3: Up arrow at first line navigates history
- [ ] AC-6.4: `/` at start shows command menu
- [ ] AC-6.5: `@` triggers file picker
- [ ] AC-6.6: Escape returns Cancel
- [ ] AC-6.7: Submit clears input
- [ ] AC-6.8: Attachments included in Submit result

### AC-7: List Selection Modal
- [ ] AC-7.1: Up/k moves selection up
- [ ] AC-7.2: Down/j moves selection down
- [ ] AC-7.3: Enter confirms selection
- [ ] AC-7.4: Escape cancels
- [ ] AC-7.5: Typing filters list
- [ ] AC-7.6: Filter is case-insensitive
- [ ] AC-7.7: Empty filter shows all items
- [ ] AC-7.8: Selection wraps at boundaries (optional)

### AC-8: Approval Modal
- [ ] AC-8.1: y/Enter approves
- [ ] AC-8.2: n/Escape rejects
- [ ] AC-8.3: a (if enabled) approves always
- [ ] AC-8.4: e (if enabled) triggers edit
- [ ] AC-8.5: Action preview displayed when provided
- [ ] AC-8.6: Keyboard shortcuts displayed

### AC-9: Status Indicators
- [ ] AC-9.1: Spinner `tick()` advances frame
- [ ] AC-9.2: Spinner wraps to first frame
- [ ] AC-9.3: Progress bar renders proportional fill
- [ ] AC-9.4: Progress bar clamps to 0.0-1.0
- [ ] AC-9.5: Labels display correctly

---

## 5. Testing Requirements

### 5.1 Text Buffer Unit Tests

```
test_buffer_new_empty
test_buffer_from_str_single_line
test_buffer_from_str_multi_line
test_buffer_content_roundtrip
test_buffer_insert_char
test_buffer_insert_char_middle
test_buffer_insert_str
test_buffer_insert_str_multiline
test_buffer_insert_newline
test_buffer_delete_backward_middle
test_buffer_delete_backward_line_start
test_buffer_delete_backward_buffer_start
test_buffer_delete_forward
test_buffer_delete_word_backward
test_buffer_delete_word_forward
test_buffer_delete_line
test_buffer_clear
test_buffer_dirty_flag
```

### 5.2 Cursor Movement Unit Tests

```
test_cursor_move_left
test_cursor_move_left_at_start
test_cursor_move_right
test_cursor_move_right_at_end
test_cursor_move_up
test_cursor_move_up_at_first_line
test_cursor_move_down
test_cursor_move_down_at_last_line
test_cursor_move_word_left
test_cursor_move_word_right
test_cursor_move_to_line_start
test_cursor_move_to_line_end
test_cursor_move_to_buffer_start
test_cursor_move_to_buffer_end
test_cursor_at_start
test_cursor_at_end
test_cursor_at_line_start
test_cursor_at_line_end
```

### 5.3 Selection Unit Tests

```
test_selection_none_initially
test_selection_select_all
test_selection_selected_text
test_selection_delete_selection
test_selection_replace_on_insert
test_selection_extend_left
test_selection_extend_right
test_selection_extend_up
test_selection_extend_down
test_selection_clear_on_move
```

### 5.4 Textarea Widget Tests

```
test_textarea_new
test_textarea_single_line
test_textarea_with_placeholder
test_textarea_handle_key_char
test_textarea_handle_key_backspace
test_textarea_handle_key_enter_multiline
test_textarea_handle_key_enter_singleline
test_textarea_handle_paste
test_textarea_scroll_on_insert
test_textarea_desired_height
test_textarea_render_with_cursor
test_textarea_render_placeholder
```

### 5.5 Input History Unit Tests

```
test_history_new
test_history_push
test_history_push_max_size
test_history_previous
test_history_previous_preserves_current
test_history_next
test_history_reset_position
test_history_is_navigating
test_history_empty_previous_returns_none
```

### 5.6 Composer Widget Tests

```
test_composer_new
test_composer_submit_on_enter
test_composer_newline_on_shift_enter
test_composer_history_navigation
test_composer_command_detection
test_composer_file_attachment
test_composer_image_attachment
test_composer_clear_on_submit
test_composer_cancel_on_escape
```

### 5.7 Modal Tests

```
test_list_selection_new
test_list_selection_move_up
test_list_selection_move_down
test_list_selection_confirm
test_list_selection_cancel
test_list_selection_filter
test_list_selection_filter_case_insensitive
test_list_selection_scroll

test_approval_new
test_approval_approve_y
test_approval_approve_enter
test_approval_reject_n
test_approval_reject_escape
test_approval_always_option
test_approval_edit_option

test_feedback_new
test_feedback_text_entry
test_feedback_category_selection
test_feedback_submit
test_feedback_cancel
```

### 5.8 Status Indicator Tests

```
test_spinner_new
test_spinner_tick
test_spinner_tick_wraps
test_spinner_render

test_progress_new
test_progress_set_progress
test_progress_clamp_min
test_progress_clamp_max
test_progress_render_empty
test_progress_render_full
test_progress_render_half
```

### 5.9 Unicode Tests

```
test_buffer_emoji_insert
test_buffer_emoji_delete
test_buffer_emoji_cursor_movement
test_buffer_cjk_characters
test_buffer_combining_characters
test_buffer_rtl_text
test_textarea_emoji_width
```

### 5.10 Property-Based Tests

```
proptest_buffer_content_roundtrip
proptest_cursor_always_valid
proptest_selection_always_valid
proptest_history_size_bounded
proptest_filter_subset_of_items
```

---

## 6. Error Handling

### 6.1 No Panics Policy

All operations are infallible:

| Situation | Behavior |
|-----------|----------|
| Cursor movement at boundary | No-op |
| Delete at buffer start | No-op |
| Insert empty string | No-op |
| History navigation when empty | Return None |
| Filter with no matches | Show empty list |
| Invalid cursor position | Clamp to valid |

### 6.2 Edge Cases

| Edge Case | Handling |
|-----------|----------|
| Empty buffer | Valid state, cursor at (0, 0) |
| Single empty line | Valid state |
| Selection across lines | Handle line joins correctly |
| Very long lines | No artificial limits |
| Very long history | Enforce max_size |

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tui-core` | workspace | InputHandler, ModalView, Renderable traits |
| `tui-render` | workspace | Word wrapping for display |
| `ratatui` | 0.28+ | Rect, Buffer, Line, Style |
| `crossterm` | 0.28+ | KeyEvent, KeyCode, KeyModifiers |
| `unicode-width` | 0.1+ | Display width calculation |
| `unicode-segmentation` | 1.0+ | Grapheme cluster iteration |

### 7.2 Forbidden Dependencies

- No async runtime
- No file system access
- No network access
- No clipboard access (handled by app layer)

---

## 8. File Structure

```
tui-input/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports
    │
    ├── textarea/
    │   ├── mod.rs              # Textarea widget
    │   ├── buffer.rs           # TextBuffer
    │   ├── cursor.rs           # CursorPosition, movement
    │   └── selection.rs        # Selection handling
    │
    ├── composer/
    │   ├── mod.rs              # Composer widget
    │   ├── history.rs          # InputHistory
    │   ├── commands.rs         # Slash command parsing
    │   └── attachments.rs      # Attachment types
    │
    ├── modal/
    │   ├── mod.rs              # Common modal utilities
    │   ├── list_selection.rs   # ListSelectionModal
    │   ├── approval.rs         # ApprovalModal
    │   └── feedback.rs         # FeedbackModal
    │
    ├── popup/
    │   ├── mod.rs
    │   ├── command.rs          # CommandPopup
    │   └── file_picker.rs      # FilePickerPopup
    │
    └── status/
        ├── mod.rs
        ├── spinner.rs          # Spinner
        └── progress.rs         # ProgressBar
```

---

## 9. Thread Safety Requirements

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `TextBuffer` | Yes | No | Contains mutable string data |
| `Textarea` | Yes | No | Contains TextBuffer |
| `InputHistory` | Yes | No | Contains VecDeque |
| `Composer` | Yes | No | Contains Textarea + History |
| `ListSelectionModal` | Yes | No | Contains selection state |
| `Spinner` | Yes | Yes | Animation state is simple |

---

## 10. Performance Requirements

- PR-1: Single character insertion MUST be O(n) where n = line length
- PR-2: Cursor movement MUST be O(1)
- PR-3: Line access by index MUST be O(1)
- PR-4: Filter application SHOULD be O(n * m) where n = items, m = filter length
- PR-5: History push/pop MUST be O(1) amortized
- PR-6: Widget render MUST be O(visible lines)

---

## 11. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
