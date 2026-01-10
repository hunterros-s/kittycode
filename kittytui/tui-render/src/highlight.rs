use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use std::sync::OnceLock;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

const HIGHLIGHT_NAMES: &[&str] = &[
    "keyword",
    "function",
    "type",
    "string",
    "number",
    "comment",
    "operator",
    "variable",
    "constant",
    "property",
    "punctuation",
    "attribute",
];

fn style_for_highlight(h: Highlight) -> Style {
    match HIGHLIGHT_NAMES.get(h.0) {
        Some(&"keyword") => Style::new().fg(Color::Magenta),
        Some(&"function") => Style::new().fg(Color::Blue),
        Some(&"type") => Style::new().fg(Color::Yellow),
        Some(&"string") => Style::new().fg(Color::Green),
        Some(&"number") => Style::new().fg(Color::Cyan),
        Some(&"comment") => Style::new().dim(),
        Some(&"operator") => Style::new().dim(),
        Some(&"constant") => Style::new().fg(Color::Cyan),
        Some(&"attribute") => Style::new().fg(Color::Yellow),
        _ => Style::default(),
    }
}

pub fn highlight_code(source: &str, language: &str) -> Vec<Line<'static>> {
    let Some(config) = get_config(language) else {
        return fallback_lines(source);
    };

    let mut highlighter = Highlighter::new();
    let Ok(highlights) = highlighter.highlight(config, source.as_bytes(), None, |_| None) else {
        return fallback_lines(source);
    };

    let mut lines: Vec<Line<'static>> = vec![Line::default()];
    let mut style_stack: Vec<Style> = vec![];

    for event in highlights.flatten() {
        match event {
            HighlightEvent::HighlightStart(h) => {
                style_stack.push(style_for_highlight(h));
            }
            HighlightEvent::HighlightEnd => {
                style_stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                let style = style_stack.last().copied().unwrap_or_default();
                let text = &source[start..end];

                for (i, part) in text.split('\n').enumerate() {
                    if i > 0 {
                        lines.push(Line::default());
                    }
                    if !part.is_empty() {
                        if let Some(line) = lines.last_mut() {
                            line.spans.push(Span::styled(part.to_string(), style));
                        }
                    }
                }
            }
        }
    }

    lines
}

fn fallback_lines(source: &str) -> Vec<Line<'static>> {
    source.lines().map(|l| Line::raw(l.to_string())).collect()
}

fn get_config(language: &str) -> Option<&'static HighlightConfiguration> {
    static CONFIGS: OnceLock<Vec<(&'static str, HighlightConfiguration)>> = OnceLock::new();

    let configs = CONFIGS.get_or_init(|| {
        let mut v = Vec::new();

        // Rust
        if let Ok(mut c) = HighlightConfiguration::new(
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            "",
            "",
        ) {
            c.configure(HIGHLIGHT_NAMES);
            v.push(("rust", c));
        }

        // Python
        if let Ok(mut c) = HighlightConfiguration::new(
            tree_sitter_python::LANGUAGE.into(),
            "python",
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        ) {
            c.configure(HIGHLIGHT_NAMES);
            v.push(("python", c));
        }

        // JavaScript
        if let Ok(mut c) = HighlightConfiguration::new(
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTIONS_QUERY,
            tree_sitter_javascript::LOCALS_QUERY,
        ) {
            c.configure(HIGHLIGHT_NAMES);
            v.push(("javascript", c));
        }

        // Bash
        if let Ok(mut c) = HighlightConfiguration::new(
            tree_sitter_bash::LANGUAGE.into(),
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        ) {
            c.configure(HIGHLIGHT_NAMES);
            v.push(("bash", c));
        }

        // JSON
        if let Ok(mut c) = HighlightConfiguration::new(
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        ) {
            c.configure(HIGHLIGHT_NAMES);
            v.push(("json", c));
        }

        v
    });

    let lang = normalize_language(language);
    configs.iter().find(|(name, _)| *name == lang).map(|(_, c)| c)
}

fn normalize_language(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" | "typescript" => "javascript", // TypeScript uses JS highlighting for now
        "sh" | "shell" | "zsh" => "bash",
        _ => lang,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let code = r#"fn main() {
    println!("hello");
}"#;
        let lines = highlight_code(code, "rust");
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_highlight_python() {
        let code = "def hello():\n    print('world')";
        let lines = highlight_code(code, "python");
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_highlight_unknown_language() {
        let code = "some code";
        let lines = highlight_code(code, "unknown_lang");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "some code");
    }

    #[test]
    fn test_normalize_language() {
        assert_eq!(normalize_language("rs"), "rust");
        assert_eq!(normalize_language("py"), "python");
        assert_eq!(normalize_language("js"), "javascript");
        assert_eq!(normalize_language("sh"), "bash");
        assert_eq!(normalize_language("rust"), "rust");
    }

    #[test]
    fn test_fallback_lines() {
        let lines = fallback_lines("line1\nline2\nline3");
        assert_eq!(lines.len(), 3);
    }
}
