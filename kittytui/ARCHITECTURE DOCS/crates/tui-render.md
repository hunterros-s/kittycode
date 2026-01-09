# tui-render Technical Specification

## Overview

**Crate Name**: `tui-render`
**Purpose**: Convert text (markdown, code, diffs) to styled terminal lines
**Dependencies**: `tui-core`, `ratatui`, `pulldown-cmark`, `unicode-width`
**Dependents**: `tui-cells`, `tui-input`, `tui-app`

---

## 1. Requirements

### 1.1 Functional Requirements

#### FR-1: Markdown Rendering
- FR-1.1: Parse CommonMark-compliant markdown
- FR-1.2: Render headings (H1-H6) with distinct styles
- FR-1.3: Render emphasis (italic), strong (bold), strikethrough
- FR-1.4: Render inline code with distinct style
- FR-1.5: Render code blocks with language hint preservation
- FR-1.6: Render blockquotes with visual indicator (│ prefix)
- FR-1.7: Render ordered and unordered lists with proper indentation
- FR-1.8: Render links with underline style
- FR-1.9: Render horizontal rules
- FR-1.10: Support nested structures (lists in blockquotes, etc.)
- FR-1.11: Support optional width-based word wrapping

#### FR-2: Word Wrapping
- FR-2.1: Wrap text at word boundaries when possible
- FR-2.2: Support initial indent (first line) and subsequent indent (continuation lines)
- FR-2.3: Preserve styling across wrapped lines
- FR-2.4: Handle words longer than available width (force break)
- FR-2.5: Handle unicode characters correctly (emoji, CJK)
- FR-2.6: Provide live wrapping utilities for streaming content

#### FR-3: Syntax Highlighting
- FR-3.1: Highlight bash/shell scripts (keywords, variables, strings, comments)
- FR-3.2: Provide generic fallback highlighting
- FR-3.3: Support language detection from code fence hints
- FR-3.4: Preserve whitespace in code blocks

#### FR-4: Diff Rendering
- FR-4.1: Render file change summaries (added, modified, deleted, renamed)
- FR-4.2: Show addition/deletion counts per file
- FR-4.3: Show total change statistics
- FR-4.4: Support path relativization

#### FR-5: Line Utilities
- FR-5.1: Prefix lines with styled content
- FR-5.2: Convert borrowed lines to owned (`'static`)
- FR-5.3: Path formatting (relativize to home, truncation)

### 1.2 Non-Functional Requirements

- NFR-1: All functions MUST be pure (no side effects)
- NFR-2: All functions MUST be stateless
- NFR-3: All output MUST use `'static` lifetime
- NFR-4: No terminal I/O (only produce ratatui types)
- NFR-5: No async operations

---

## 2. Non-Requirements (Out of Scope)

- NR-1: Full syntax highlighting for all languages (only bash + generic)
- NR-2: Markdown extensions (tables, footnotes, task lists)
- NR-3: HTML rendering within markdown
- NR-4: Image rendering
- NR-5: Actual diff computation (only rendering of pre-computed diffs)

---

## 3. Interface Specification

### 3.1 Markdown API

```rust
/// Render markdown without wrapping
pub fn render_markdown(text: &str) -> Text<'static>;

/// Render markdown with optional word wrapping
pub fn render_markdown_with_width(text: &str, width: Option<usize>) -> Text<'static>;

/// Configurable markdown styles
pub struct MarkdownStyles {
    pub heading1: Style,
    pub heading2: Style,
    pub heading3: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub strikethrough: Style,
    pub code_inline: Style,
    pub code_block: Style,
    pub link: Style,
    pub blockquote: Style,
    pub list_marker: Style,
}
```

### 3.2 Word Wrap API

```rust
/// Wrapping configuration
pub struct WrapOptions {
    pub initial_indent: String,
    pub subsequent_indent: String,
}

/// Wrap a single line
pub fn word_wrap(line: Line<'static>, width: usize, opts: WrapOptions) -> Vec<Line<'static>>;

/// Wrap multiple lines
pub fn word_wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>>;

/// Extract prefix by display width (for streaming)
pub fn take_prefix_by_width(s: &str, width: usize) -> (&str, &str);

/// Split at word boundary within width
pub fn split_at_word_boundary(s: &str, width: usize) -> (&str, &str);
```

### 3.3 Syntax Highlighting API

```rust
/// Highlight bash/shell code
pub fn highlight_bash(code: &str) -> Vec<Line<'static>>;

/// Highlight with language hint
pub fn highlight_code(code: &str, language: Option<&str>) -> Vec<Line<'static>>;
```

### 3.4 Diff Rendering API

```rust
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
}

pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
}

pub struct DiffSummary { /* ... */ }

impl DiffSummary {
    pub fn new(files: Vec<FileChange>, base_path: Option<String>) -> Self;
    pub fn render(&self, width: u16) -> Vec<Line<'static>>;
    pub fn file_count(&self) -> usize;
    pub fn total_additions(&self) -> usize;
    pub fn total_deletions(&self) -> usize;
}
```

### 3.5 Line Utilities API

```rust
/// Prefix each line with styled string
pub fn prefix_lines(lines: Vec<Line<'_>>, prefix: &str, style: Style) -> Vec<Line<'_>>;

/// Convert to owned/static lifetime
pub fn lines_to_static(lines: Vec<Line<'_>>) -> Vec<Line<'static>>;

/// Append with lifetime conversion
pub fn push_owned_lines(dest: &mut Vec<Line<'static>>, lines: Vec<Line<'_>>);

/// Format path relative to home
pub fn relativize_to_home(path: &Path) -> String;

/// Format and truncate path
pub fn format_path(path: &Path, max_width: usize) -> String;
```

---

## 4. Acceptance Criteria

### AC-1: Markdown Rendering
- [ ] AC-1.1: `# Heading` renders with heading1 style
- [ ] AC-1.2: `## Heading` renders with heading2 style
- [ ] AC-1.3: `*italic*` and `_italic_` render with emphasis style
- [ ] AC-1.4: `**bold**` and `__bold__` render with strong style
- [ ] AC-1.5: `` `code` `` renders with code_inline style
- [ ] AC-1.6: Fenced code blocks preserve all whitespace
- [ ] AC-1.7: `> quote` renders with blockquote style and │ prefix
- [ ] AC-1.8: `- item` renders with bullet marker
- [ ] AC-1.9: `1. item` renders with number marker, incrementing correctly
- [ ] AC-1.10: Nested lists increase indentation
- [ ] AC-1.11: `---` renders as horizontal rule
- [ ] AC-1.12: `[text](url)` renders with link style
- [ ] AC-1.13: Empty input returns empty Text
- [ ] AC-1.14: Width=None produces no wrapping
- [ ] AC-1.15: Width=Some(n) wraps lines to n columns

### AC-2: Word Wrapping
- [ ] AC-2.1: Words are not broken mid-word when they fit
- [ ] AC-2.2: Words longer than width ARE broken
- [ ] AC-2.3: initial_indent appears on first line only
- [ ] AC-2.4: subsequent_indent appears on continuation lines
- [ ] AC-2.5: Styles are preserved across line breaks
- [ ] AC-2.6: Empty line input returns single empty line
- [ ] AC-2.7: Width of 0 returns original content (no infinite loop)
- [ ] AC-2.8: Unicode width is used, not byte length
- [ ] AC-2.9: Emoji (width 2) handled correctly
- [ ] AC-2.10: CJK characters (width 2) handled correctly

### AC-3: Syntax Highlighting
- [ ] AC-3.1: Bash comments (`#...`) render in comment style
- [ ] AC-3.2: Bash strings (`"..."`, `'...'`) render in string style
- [ ] AC-3.3: Bash variables (`$VAR`, `${VAR}`) render in variable style
- [ ] AC-3.4: Bash keywords (`if`, `then`, `fi`, etc.) render in keyword style
- [ ] AC-3.5: Unknown language falls back to generic highlighting
- [ ] AC-3.6: Empty code returns empty vec
- [ ] AC-3.7: Whitespace-only lines are preserved

### AC-4: Diff Rendering
- [ ] AC-4.1: Added files show `+` indicator in green
- [ ] AC-4.2: Modified files show `~` indicator in yellow
- [ ] AC-4.3: Deleted files show `-` indicator in red
- [ ] AC-4.4: Renamed files show `→` indicator in cyan
- [ ] AC-4.5: Addition count shows in green
- [ ] AC-4.6: Deletion count shows in red
- [ ] AC-4.7: Header shows total file count and changes
- [ ] AC-4.8: Paths are relativized when base_path provided
- [ ] AC-4.9: Empty file list renders header only

### AC-5: Line Utilities
- [ ] AC-5.1: `prefix_lines` adds prefix to every line
- [ ] AC-5.2: `prefix_lines` applies style to prefix only
- [ ] AC-5.3: `lines_to_static` produces owned strings
- [ ] AC-5.4: `relativize_to_home` replaces home dir with `~`
- [ ] AC-5.5: `format_path` truncates long paths
- [ ] AC-5.6: `format_path` preserves short paths

---

## 5. Testing Requirements

### 5.1 Markdown Unit Tests

```
test_markdown_empty_input
test_markdown_plain_text_only
test_markdown_heading_h1
test_markdown_heading_h2
test_markdown_heading_h3_through_h6
test_markdown_emphasis_asterisk
test_markdown_emphasis_underscore
test_markdown_strong_asterisk
test_markdown_strong_underscore
test_markdown_strikethrough
test_markdown_inline_code
test_markdown_code_block_no_language
test_markdown_code_block_with_language
test_markdown_code_block_preserves_whitespace
test_markdown_blockquote_single_line
test_markdown_blockquote_multiple_lines
test_markdown_blockquote_nested
test_markdown_unordered_list
test_markdown_ordered_list
test_markdown_nested_list
test_markdown_horizontal_rule
test_markdown_link
test_markdown_combined_formatting
test_markdown_with_width_wraps
test_markdown_without_width_no_wrap
```

### 5.2 Word Wrap Unit Tests

```
test_wrap_short_line_no_wrap
test_wrap_exact_width_no_wrap
test_wrap_single_word_overflow
test_wrap_multiple_words
test_wrap_long_word_force_break
test_wrap_preserves_style
test_wrap_initial_indent
test_wrap_subsequent_indent
test_wrap_both_indents
test_wrap_empty_line
test_wrap_whitespace_only
test_wrap_width_zero_no_panic
test_wrap_width_one
test_wrap_unicode_emoji
test_wrap_unicode_cjk
test_wrap_combining_characters
test_take_prefix_exact
test_take_prefix_partial
test_take_prefix_empty
test_split_word_boundary_found
test_split_word_boundary_not_found
```

### 5.3 Syntax Highlighting Unit Tests

```
test_highlight_bash_comment
test_highlight_bash_double_quote_string
test_highlight_bash_single_quote_string
test_highlight_bash_variable_simple
test_highlight_bash_variable_braced
test_highlight_bash_keyword_if
test_highlight_bash_keyword_for
test_highlight_bash_builtin_cd
test_highlight_bash_builtin_echo
test_highlight_bash_mixed_line
test_highlight_bash_empty
test_highlight_bash_multiline
test_highlight_code_bash_hint
test_highlight_code_sh_hint
test_highlight_code_unknown_language
test_highlight_code_no_language
```

### 5.4 Diff Rendering Unit Tests

```
test_diff_empty_files
test_diff_single_added_file
test_diff_single_modified_file
test_diff_single_deleted_file
test_diff_renamed_file
test_diff_multiple_files
test_diff_with_base_path
test_diff_totals_calculation
test_diff_zero_additions
test_diff_zero_deletions
```

### 5.5 Property-Based Tests

```
proptest_wrap_output_width_never_exceeds_input_width
proptest_wrap_total_content_preserved
proptest_markdown_no_panic_on_arbitrary_input
proptest_highlight_no_panic_on_arbitrary_input
```

### 5.6 Snapshot Tests

```
snapshot_markdown_complex_document
snapshot_markdown_code_heavy_document
snapshot_diff_typical_output
snapshot_bash_script_highlighting
```

---

## 6. Error Handling

This crate has NO error returns - all functions are infallible.

| Situation | Behavior |
|-----------|----------|
| Empty input | Return empty output |
| Invalid markdown | Best-effort render |
| Unknown language | Use generic highlighting |
| Width = 0 | Return original content |
| Malformed unicode | Use replacement character |

---

## 7. Dependencies

### 7.1 Required Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tui-core` | workspace | Traits, utilities |
| `ratatui` | 0.28+ | Line, Span, Text, Style types |
| `pulldown-cmark` | 0.11+ | Markdown parsing |
| `unicode-width` | 0.1+ | Display width calculation |
| `textwrap` | 0.16+ | Word wrapping algorithm |

### 7.2 Optional Dependencies

| Crate | Feature | Purpose |
|-------|---------|---------|
| `dirs` | - | Home directory detection |

### 7.3 Forbidden Dependencies

- No async runtime
- No terminal I/O
- No file I/O (except `dirs` for home path)
- No network I/O

---

## 8. File Structure

```
tui-render/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports
    ├── markdown/
    │   ├── mod.rs              # Public API
    │   ├── parser.rs           # pulldown-cmark wrapper
    │   ├── writer.rs           # Event -> Line conversion
    │   └── styles.rs           # MarkdownStyles struct
    ├── wrap/
    │   ├── mod.rs              # Public API
    │   ├── word_wrap.rs        # Main wrapping algorithm
    │   └── live_wrap.rs        # Streaming utilities
    ├── highlight/
    │   ├── mod.rs              # Public API
    │   ├── bash.rs             # Bash highlighting
    │   └── generic.rs          # Fallback highlighting
    ├── diff/
    │   ├── mod.rs              # Public API
    │   └── render.rs           # DiffSummary implementation
    └── format/
        ├── mod.rs              # Public API
        ├── lines.rs            # Line utilities
        └── path.rs             # Path formatting
```

---

## 9. Performance Requirements

- PR-1: Markdown rendering MUST be O(n) in input length
- PR-2: Word wrapping MUST be O(n) in input length
- PR-3: No allocations for unchanged lines in wrapping
- PR-4: Syntax highlighting MUST be single-pass

---

## 10. Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial specification |
