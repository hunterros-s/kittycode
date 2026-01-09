# TUI Architecture

## Overview

A modular terminal user interface framework for building conversational AI applications. The architecture separates concerns into six crates with clear boundaries and dependencies.

## Crate Structure

```
                      tui-app
                     /   |   \
                    /    |    \
             tui-input  tui-cells  tui-terminal
                   \     |     /
                    \    |    /
                     tui-render
                         |
                      tui-core
```

## Crate Responsibilities

| Crate | Purpose | Key Abstractions |
|-------|---------|------------------|
| **tui-core** | Traits and primitives | `CellData`, `Renderable`, `InputHandler`, `ProtocolEvent` |
| **tui-render** | Text rendering (pure functions) | `render_markdown()`, `word_wrap()`, `highlight_code()` |
| **tui-terminal** | Terminal lifecycle and I/O | `Tui`, `TuiEvent`, `FrameRequester`, `EventStream` |
| **tui-cells** | Conversation history cells | `UserMessageData`, `AgentMessageData`, `StreamController` |
| **tui-input** | User input widgets | `TextBuffer`, `Textarea`, `Composer`, modals |
| **tui-app** | Application orchestration | `App`, `EventLoop`, `AppState`, protocol bridge |

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATION LAYER                        │
│                        tui-app                              │
│  Event loop, protocol handling, state management, views     │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                      WIDGET LAYER                           │
│              tui-input    │    tui-cells                    │
│  Input widgets, modals    │    History cell types           │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                     RENDER LAYER                            │
│                       tui-render                            │
│  Markdown, word wrap, syntax highlighting, diff display     │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   TERMINAL LAYER                            │
│                      tui-terminal                           │
│  Raw terminal, events, frame scheduling, viewport           │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   FOUNDATION LAYER                          │
│                       tui-core                              │
│  Traits, primitives, utilities (no implementations)         │
└─────────────────────────────────────────────────────────────┘
```

## Dependency Rules

1. **Downward only**: Crates may only depend on crates below them in the hierarchy
2. **No cycles**: The dependency graph is strictly acyclic
3. **Core has no dependencies**: `tui-core` depends only on external crates (ratatui, crossterm)
4. **Siblings don't depend on each other**: `tui-input` and `tui-cells` are independent

## Data Flow

```
User Input                          Protocol Events
    │                                      │
    ▼                                      ▼
┌─────────┐                          ┌─────────────┐
│tui-input│                          │ tui-cells   │
└────┬────┘                          └──────┬──────┘
     │                                      │
     │         ┌─────────────┐              │
     └────────►│   tui-app   │◄─────────────┘
               │ (event loop)│
               └──────┬──────┘
                      │
                      ▼
               ┌─────────────┐
               │tui-terminal │
               │  (render)   │
               └─────────────┘
```

## Key Design Principles

### Separation of Data and Rendering
- `CellData` trait (tui-core): Domain data, no rendering logic
- `CellRenderer` trait (tui-core): Rendering logic, separate from data
- Implementations live in `tui-cells`

### Trait-Based Abstractions
- `InputHandler`: Standardized keyboard/paste handling
- `Renderable`: Standardized component rendering
- `ModalView`: Standardized modal lifecycle
- `Protocol*` traits: Generic backend integration

### Pure Functions for Rendering
- `tui-render` contains only pure, stateless functions
- No side effects, no terminal I/O
- Easy to test and compose

### Single-Threaded Event Loop
- All UI updates happen on main thread
- Protocol communication via async channels
- Frame scheduling decoupled from event processing

## Detailed Specifications

Each crate has a comprehensive technical specification:

| Spec | Contents |
|------|----------|
| [tui-core.md](crates/tui-core.md) | Trait definitions, utilities, primitives |
| [tui-render.md](crates/tui-render.md) | Markdown, wrapping, highlighting APIs |
| [tui-terminal.md](crates/tui-terminal.md) | Terminal lifecycle, events, viewport |
| [tui-cells.md](crates/tui-cells.md) | Cell types, renderers, streaming |
| [tui-input.md](crates/tui-input.md) | Text editing, composer, modals |
| [tui-app.md](crates/tui-app.md) | Application structure, event handling |

Each specification includes:
- Functional and non-functional requirements
- Interface specifications with Rust signatures
- Acceptance criteria
- Testing requirements
- Error handling
- File structure

## External Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | Terminal UI framework (Buffer, Rect, Style, Line) |
| `crossterm` | Cross-platform terminal I/O |
| `tokio` | Async runtime and channels |
| `pulldown-cmark` | Markdown parsing |
| `unicode-width` | Display width calculation |

## Thread Safety Summary

| Type | Send | Sync | Location |
|------|------|------|----------|
| `CellData` implementations | Yes | Yes | tui-cells |
| `FrameRequester` | Yes | Yes | tui-terminal |
| `Tui` | No | No | tui-terminal |
| `TextBuffer` | Yes | No | tui-input |
| `App` | No | No | tui-app |
