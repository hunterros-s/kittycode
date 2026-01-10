//! Demo of all cell types - run with: cargo run -p tui-app --example cell_demo

use std::time::Duration;

use crossterm::terminal;
use tui_cells::{Cell, ErrorCell, ToolCallCell, ToolOutput, ToolStatus};

fn main() {
    let width = terminal::size().map(|(w, _)| w).unwrap_or(80);

    println!("\n=== Cell Types Demo ===\n");

    // ErrorCell - simple
    println!("ErrorCell (simple):");
    let error = ErrorCell::new("Connection refused");
    for line in error.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ErrorCell - with details
    println!("ErrorCell (with details):");
    let error = ErrorCell::with_details(
        "API request failed",
        "Status: 429 Too Many Requests\nRetry-After: 60 seconds",
    );
    for line in error.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - pending
    println!("ToolCallCell (pending):");
    let tool = ToolCallCell::pending("bash", "npm install");
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - running
    println!("ToolCallCell (running):");
    let tool = ToolCallCell::running("bash", "npm test");
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - complete success
    println!("ToolCallCell (success):");
    let mut tool = ToolCallCell::new("bash", "npm install");
    tool.status = ToolStatus::Complete {
        exit_code: 0,
        duration: Duration::from_secs_f32(2.3),
    };
    tool.output = Some(ToolOutput {
        stdout: "added 150 packages in 2.3s".to_string(),
        stderr: String::new(),
    });
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - complete failure
    println!("ToolCallCell (failure):");
    let mut tool = ToolCallCell::new("bash", "npm test");
    tool.status = ToolStatus::Complete {
        exit_code: 1,
        duration: Duration::from_secs_f32(5.7),
    };
    tool.output = Some(ToolOutput {
        stdout: String::new(),
        stderr: "Error: 3 tests failed\n  - auth.test.ts\n  - user.test.ts\n  - api.test.ts"
            .to_string(),
    });
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - truncated output
    println!("ToolCallCell (truncated output):");
    let mut tool = ToolCallCell::new("bash", "cat long_file.txt");
    tool.status = ToolStatus::Complete {
        exit_code: 0,
        duration: Duration::from_secs_f32(0.1),
    };
    tool.output = Some(ToolOutput {
        stdout: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10"
            .to_string(),
        stderr: String::new(),
    });
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();

    // ToolCallCell - no output
    println!("ToolCallCell (no output):");
    let mut tool = ToolCallCell::new("bash", "mkdir -p src/components");
    tool.status = ToolStatus::Complete {
        exit_code: 0,
        duration: Duration::from_secs_f32(0.01),
    };
    tool.output = Some(ToolOutput::default());
    for line in tool.render_lines(width) {
        println!("  {}", line);
    }
    println!();
}
