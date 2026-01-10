use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::borrow::Cow;
use std::ops::Range;
use textwrap::Options;

/// Options for word wrapping ratatui Lines.
#[derive(Debug, Clone)]
pub struct WrapOptions<'a> {
    /// The width in columns at which the text will be wrapped.
    pub width: usize,
    /// Indentation used for the first line of output.
    pub initial_indent: Line<'a>,
    /// Indentation used for subsequent lines of output.
    pub subsequent_indent: Line<'a>,
    /// Allow long words to be broken if they cannot fit on a line.
    pub break_words: bool,
}

impl Default for WrapOptions<'_> {
    fn default() -> Self {
        Self {
            width: 80,
            initial_indent: Line::default(),
            subsequent_indent: Line::default(),
            break_words: true,
        }
    }
}

impl<'a> WrapOptions<'a> {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            ..Default::default()
        }
    }

    pub fn initial_indent(mut self, indent: Line<'a>) -> Self {
        self.initial_indent = indent;
        self
    }

    pub fn subsequent_indent(mut self, indent: Line<'a>) -> Self {
        self.subsequent_indent = indent;
        self
    }

    pub fn break_words(mut self, break_words: bool) -> Self {
        self.break_words = break_words;
        self
    }
}

impl From<usize> for WrapOptions<'_> {
    fn from(width: usize) -> Self {
        WrapOptions::new(width)
    }
}

/// Clone a borrowed ratatui `Line` into an owned `'static` line.
fn line_to_static(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|s| Span {
                style: s.style,
                content: Cow::Owned(s.content.to_string()),
            })
            .collect(),
    }
}

/// Append owned copies of borrowed lines to `out`.
fn push_owned_lines<'a>(src: &[Line<'a>], out: &mut Vec<Line<'static>>) {
    for l in src {
        out.push(line_to_static(l));
    }
}

/// Compute byte ranges for wrapped lines using textwrap.
fn wrap_ranges_trim<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut lines: Vec<Range<usize>> = Vec::new();
    for line in textwrap::wrap(text, opts).iter() {
        match line {
            Cow::Borrowed(slice) => {
                let start = unsafe { slice.as_ptr().offset_from(text.as_ptr()) as usize };
                let end = start + slice.len();
                lines.push(start..end);
            }
            Cow::Owned(_) => panic!("wrap_ranges_trim: unexpected owned string"),
        }
    }
    lines
}

/// Slice spans from a Line at arbitrary byte positions while preserving styles.
fn slice_line_spans<'a>(
    original: &'a Line<'a>,
    span_bounds: &[(Range<usize>, Style)],
    range: &Range<usize>,
) -> Line<'a> {
    let start_byte = range.start;
    let end_byte = range.end;
    let mut acc: Vec<Span<'a>> = Vec::new();

    for (i, (span_range, style)) in span_bounds.iter().enumerate() {
        let s = span_range.start;
        let e = span_range.end;

        if e <= start_byte {
            continue;
        }
        if s >= end_byte {
            break;
        }

        let seg_start = start_byte.max(s);
        let seg_end = end_byte.min(e);

        if seg_end > seg_start {
            let local_start = seg_start - s;
            let local_end = seg_end - s;
            let content = original.spans[i].content.as_ref();
            let slice = &content[local_start..local_end];
            acc.push(Span {
                style: *style,
                content: Cow::Borrowed(slice),
            });
        }

        if e >= end_byte {
            break;
        }
    }

    Line {
        style: original.style,
        alignment: original.alignment,
        spans: acc,
    }
}

/// Word wrap a single Line, preserving styles across word boundaries.
#[must_use]
pub fn word_wrap<'a, O>(line: &'a Line<'a>, width_or_options: O) -> Vec<Line<'a>>
where
    O: Into<WrapOptions<'a>>,
{
    let opts: WrapOptions<'a> = width_or_options.into();

    if opts.width == 0 {
        return vec![line.clone()];
    }

    // Flatten the line and record span byte ranges.
    let mut flat = String::new();
    let mut span_bounds = Vec::new();
    let mut acc = 0usize;
    for s in &line.spans {
        let text = s.content.as_ref();
        let start = acc;
        flat.push_str(text);
        acc += text.len();
        span_bounds.push((start..acc, s.style));
    }

    let textwrap_opts = Options::new(opts.width)
        .break_words(opts.break_words)
        .wrap_algorithm(textwrap::WrapAlgorithm::FirstFit)
        .word_separator(textwrap::WordSeparator::new())
        .word_splitter(textwrap::WordSplitter::HyphenSplitter);

    let mut out: Vec<Line<'a>> = Vec::new();

    // Compute first line range with reduced width due to initial indent.
    let initial_width_available = opts
        .width
        .saturating_sub(opts.initial_indent.width())
        .max(1);
    let initial_wrapped = wrap_ranges_trim(&flat, textwrap_opts.clone().width(initial_width_available));

    let Some(first_line_range) = initial_wrapped.first() else {
        return vec![opts.initial_indent.clone()];
    };

    // Build first wrapped line with initial indent.
    let mut first_line = opts.initial_indent.clone().style(line.style);
    {
        let sliced = slice_line_spans(line, &span_bounds, first_line_range);
        let mut spans = first_line.spans;
        spans.append(
            &mut sliced
                .spans
                .into_iter()
                .map(|s| s.patch_style(line.style))
                .collect(),
        );
        first_line.spans = spans;
        out.push(first_line);
    }

    // Wrap the remainder using subsequent indent width.
    let base = first_line_range.end;
    let skip_leading_spaces = flat[base..].chars().take_while(|c| *c == ' ').count();
    let base = base + skip_leading_spaces;

    let subsequent_width_available = opts
        .width
        .saturating_sub(opts.subsequent_indent.width())
        .max(1);
    let remaining_wrapped = wrap_ranges_trim(&flat[base..], textwrap_opts.width(subsequent_width_available));

    for r in &remaining_wrapped {
        if r.is_empty() {
            continue;
        }
        let mut subsequent_line = opts.subsequent_indent.clone().style(line.style);
        let offset_range = (r.start + base)..(r.end + base);
        let sliced = slice_line_spans(line, &span_bounds, &offset_range);
        let mut spans = subsequent_line.spans;
        spans.append(
            &mut sliced
                .spans
                .into_iter()
                .map(|s| s.patch_style(line.style))
                .collect(),
        );
        subsequent_line.spans = spans;
        out.push(subsequent_line);
    }

    out
}

/// Wrap multiple lines, applying initial indent only to the first output line.
pub fn wrap_lines<'a, I, O>(lines: I, width_or_options: O) -> Vec<Line<'static>>
where
    I: IntoIterator<Item = Line<'a>>,
    O: Into<WrapOptions<'a>>,
{
    let base_opts: WrapOptions<'a> = width_or_options.into();
    let mut out: Vec<Line<'static>> = Vec::new();

    for (idx, line) in lines.into_iter().enumerate() {
        let opts = if idx == 0 {
            base_opts.clone()
        } else {
            let mut o = base_opts.clone();
            let sub = o.subsequent_indent.clone();
            o.initial_indent = sub;
            o
        };
        let wrapped = word_wrap(&line, opts);
        push_owned_lines(&wrapped, &mut out);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Stylize};

    fn concat_line(line: &Line) -> String {
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn wrap_short_line_no_change() {
        let line = Line::from("hello");
        let wrapped = word_wrap(&line, 80);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(concat_line(&wrapped[0]), "hello");
    }

    #[test]
    fn wrap_long_line_breaks() {
        let line = Line::from("hello world");
        let wrapped = word_wrap(&line, 5);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(concat_line(&wrapped[0]), "hello");
        assert_eq!(concat_line(&wrapped[1]), "world");
    }

    #[test]
    fn wrap_preserves_style() {
        let line = Line::from(vec!["hello ".red(), "world".into()]);
        let wrapped = word_wrap(&line, 6);
        assert_eq!(wrapped.len(), 2);
        // First line should carry the red style
        assert_eq!(concat_line(&wrapped[0]), "hello");
        assert_eq!(wrapped[0].spans.len(), 1);
        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        // Second line is unstyled
        assert_eq!(concat_line(&wrapped[1]), "world");
        assert_eq!(wrapped[1].spans.len(), 1);
        assert_eq!(wrapped[1].spans[0].style.fg, None);
    }

    #[test]
    fn wrap_handles_emoji() {
        let line = Line::from("😀😀😀");
        let wrapped = word_wrap(&line, 4);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(concat_line(&wrapped[0]), "😀😀");
        assert_eq!(concat_line(&wrapped[1]), "😀");
    }

    #[test]
    fn wrap_handles_cjk() {
        let line = Line::from("日本語テスト");
        let wrapped = word_wrap(&line, 80);
        assert!(!wrapped.is_empty());
        assert!(concat_line(&wrapped[0]).contains("日本語"));
    }

    #[test]
    fn wrap_with_indent() {
        let opts = WrapOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));
        let line = Line::from("hello world foo");
        let wrapped = word_wrap(&line, opts);
        assert!(concat_line(&wrapped[0]).starts_with("- "));
        assert!(concat_line(&wrapped[1]).starts_with("  "));
        assert!(concat_line(&wrapped[2]).starts_with("  "));
        assert_eq!(concat_line(&wrapped[0]), "- hello");
        assert_eq!(concat_line(&wrapped[1]), "  world");
        assert_eq!(concat_line(&wrapped[2]), "  foo");
    }

    #[test]
    fn wrap_empty_line() {
        let line = Line::from("");
        let wrapped = word_wrap(&line, 80);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(concat_line(&wrapped[0]), "");
    }

    #[test]
    fn wrap_zero_width() {
        let line = Line::from("hello");
        let wrapped = word_wrap(&line, 0);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(concat_line(&wrapped[0]), "hello");
    }

    #[test]
    fn wrap_single_long_word() {
        let opts = WrapOptions::new(5).break_words(false);
        let line = Line::from("supercalifragilistic");
        let wrapped = word_wrap(&line, opts);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(concat_line(&wrapped[0]), "supercalifragilistic");
    }

    #[test]
    fn wrap_multiple_lines() {
        let lines = vec![
            Line::from("first line here"),
            Line::from("second line here"),
        ];
        let wrapped = wrap_lines(lines, 10);
        assert!(wrapped.len() >= 2);
    }

    #[test]
    fn styled_split_within_span_preserves_style() {
        let line = Line::from(vec!["abcd".red()]);
        let wrapped = word_wrap(&line, 2);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].spans.len(), 1);
        assert_eq!(wrapped[1].spans.len(), 1);
        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(wrapped[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(concat_line(&wrapped[0]), "ab");
        assert_eq!(concat_line(&wrapped[1]), "cd");
    }

    #[test]
    fn wrap_lines_applies_initial_indent_only_once() {
        let opts = WrapOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));

        let lines = vec![Line::from("hello world"), Line::from("foo bar baz")];
        let wrapped = wrap_lines(lines, opts);

        let rendered: Vec<String> = wrapped.iter().map(concat_line).collect();
        assert!(rendered[0].starts_with("- "));
        for r in rendered.iter().skip(1) {
            assert!(r.starts_with("  "));
        }
    }

    #[test]
    fn hyphen_splitter_breaks_at_hyphen() {
        let line = Line::from("hello-world");
        let wrapped = word_wrap(&line, 7);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(concat_line(&wrapped[0]), "hello-");
        assert_eq!(concat_line(&wrapped[1]), "world");
    }
}
