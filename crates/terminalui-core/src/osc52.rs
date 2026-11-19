//! OSC 52 clipboard writes: the escape sequence terminals use to receive
//! text copied in the application.
//!
//! Best effort: only when stdout is a terminal, and only terminals that
//! implement the sequence see anything; everything else leaves the user
//! unaffected.

use std::io::{IsTerminal, Write};

/// Sends `text` to the system clipboard via OSC 52.
///
/// # Errors
/// Returns the write error when stdout is a terminal and the write fails.
/// Not being a terminal, or the write itself, is otherwise not an error the
/// caller can act on.
pub fn write(text: &str) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return Ok(());
    }
    let mut out = stdout.lock();
    write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()))?;
    out.flush()
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(*chunk.get(1).unwrap_or(&0));
        let third = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (first << 16) | (second << 8) | third;
        out.push(ALPHABET[(n >> 18) as usize & 0x3F] as char);
        out.push(ALPHABET[(n >> 12) as usize & 0x3F] as char);
        out.push(if chunk.len() < 2 {
            '='
        } else {
            ALPHABET[(n >> 6) as usize & 0x3F] as char
        });
        out.push(if chunk.len() < 3 {
            '='
        } else {
            ALPHABET[n as usize & 0x3F] as char
        });
    }
    out
}

#[cfg(test)]
#[path = "osc52_tests.rs"]
mod tests;
