//! Interactive post-extract prompt (TTY-only).
//!
//! After markdown is printed to stdout, this module offers the user a single-
//! keypress menu to copy to clipboard or write to a file. When stdout is not a
//! TTY (piped or redirected), the function returns immediately without printing
//! anything, keeping `pmtx extract | cat` clean.

use anyhow::Result;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, IsTerminal, Write};

/// Show an interactive prompt if stdout is a TTY.
///
/// - `c` → copy markdown to clipboard
/// - `w` → write markdown to PROMPTS.md in the current directory
/// - anything else (including Enter) → exit silently
pub fn maybe_prompt(markdown: &str) -> Result<()> {
    // Only show when connected to a terminal
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    // Check clipboard availability upfront so we can omit the option if headless
    let clipboard_ok = arboard::Clipboard::new().is_ok();

    eprintln!();
    eprintln!("┌─────────────────────────────────────────┐");
    if clipboard_ok {
        eprintln!("│  c - Copy to clipboard                  │");
    }
    eprintln!("│  w - Write to PROMPTS.md               │");
    eprintln!("│  Enter / any key - Exit                 │");
    eprintln!("└─────────────────────────────────────────┘");
    eprint!("Choice: ");
    io::stderr().flush().ok();

    enable_raw_mode()?;
    let key = read_key();
    disable_raw_mode()?;

    // Echo a newline after the hidden keypress
    eprintln!();

    match key {
        Some('c') if clipboard_ok => {
            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(markdown)) {
                Ok(()) => eprintln!("✓ Copied to clipboard!"),
                Err(e) => eprintln!("✗ Clipboard error: {e}"),
            }
        }
        Some('w') => {
            std::fs::write("PROMPTS.md", markdown)?;
            eprintln!("✓ Written to PROMPTS.md");
        }
        _ => {}
    }

    Ok(())
}

/// Read a single character keypress. Returns `None` on EOF or read error.
fn read_key() -> Option<char> {
    loop {
        match read() {
            Ok(Event::Key(KeyEvent { code, .. })) => {
                return match code {
                    KeyCode::Char(c) => Some(c),
                    KeyCode::Enter => Some('\n'),
                    _ => None,
                };
            }
            Ok(_) => continue, // skip non-key events (resize, mouse, etc.)
            Err(_) => return None,
        }
    }
}
