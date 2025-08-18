// crates/zettel-cli/src/stdin.rs - Centralized STDIN handling utility
//
// This module provides clean stdin reading that all commands can use.
// Follows Unix conventions: read from stdin when no positional args provided.

use anyhow::Result;
use std::io::{self, IsTerminal, Read};

/// Read input from stdin or use provided argument
///
/// This is the core Unix pattern: if user provides an argument, use it.
/// If not, read from stdin. This enables both interactive use and piping.
///
/// UNIX PATTERN:
/// ```bash
/// zettel id next-sibling 1a2        # Use argument
/// echo "1a2" | zettel id next-sibling  # Use stdin
/// ```
pub fn read_input_or_stdin(arg: Option<&str>) -> Result<String> {
    match arg {
        Some(value) => Ok(value.to_string()),
        None => {
            // Check if we're in a terminal without piped input
            if io::stdin().is_terminal() {
                return Err(anyhow::anyhow!(
                    "No input provided. Either provide an argument or pipe input.\n\nExamples:\n  zettel id next-sibling 1a2\n  echo \"1a2\" | zettel id next-sibling"
                ));
            }

            // Read from stdin
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;

            let input = buffer.trim().to_string();
            if input.is_empty() {
                return Err(anyhow::anyhow!("Empty input provided"));
            }

            Ok(input)
        }
    }
}

/// Read multiple lines from stdin for batch operations
///
/// Enables powerful batch processing:
/// ```bash
/// zettel list --format=json | jq -r '.[].id' | zettel id validate
/// ```
pub fn read_lines_from_stdin() -> Result<Vec<String>> {
    if io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "No piped input detected. This command expects input from stdin."
        ));
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let lines: Vec<String> = buffer
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(anyhow::anyhow!("No input lines provided"));
    }

    Ok(lines)
}

/// Read null-terminated input for safe filename handling
///
/// Supports the -0 pattern for handling filenames with spaces:
/// ```bash
/// find . -name "*.md" -print0 | zettel extract-ids -0
/// ```
pub fn read_null_terminated_input() -> Result<Vec<String>> {
    if io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "No piped input detected. This command expects null-terminated input from stdin."
        ));
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let items: Vec<String> = buffer
        .split('\0')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

    if items.is_empty() {
        return Err(anyhow::anyhow!("No input items provided"));
    }

    Ok(items)
}
