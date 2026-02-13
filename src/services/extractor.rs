use regex::Regex;
use std::collections::HashSet;

/// Extract keywords from issue text (filenames, symbols, function names)
/// Returns unique words that look like code identifiers or file names
pub fn extract_keywords(text: &str) -> Vec<String> {
    let mut keywords = HashSet::new();

    // Pattern for code-like identifiers: snake_case, camelCase, PascalCase
    let identifier_pattern = Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b").unwrap();

    for capture in identifier_pattern.find_iter(text) {
        let word = capture.as_str();

        // Filter out common English words and very short identifiers
        if word.len() >= 3 && !is_common_word(word) {
            keywords.insert(word.to_string());
        }
    }

    keywords.into_iter().collect()
}

/// Extract mentioned files from text (looks for file paths)
/// Finds patterns like: src/main.rs, lib/utils.py, components/Button.tsx
pub fn extract_mentioned_files(text: &str) -> Vec<String> {
    // TODO(human): Implement file path extraction using regex
    // Look for patterns like:
    // - src/main.rs
    // - lib/utils.py
    // - components/Button.tsx
    // - path/to/file.js
    // Common file extensions: .rs, .py, .js, .ts, .tsx, .jsx, .go, .java, .c, .cpp, .h
    let file_pattern = Regex::new(r"[\w/.-]+\.(rs|py|js|ts|tsx|jsx|go|java|c|cpp|h)\b").unwrap();
    let mut files: HashSet<String> = HashSet::new();
    
    todo!("Extract file paths from text")
}

/// Extract stack traces from issue text
/// Returns each stack trace as a separate string
pub fn extract_stack_traces(text: &str) -> Vec<String> {
    let mut traces = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

    let mut current_trace: Vec<String> = Vec::new();
    let mut in_trace = false;

    // Patterns that indicate stack trace lines
    let trace_patterns = vec![
        Regex::new(r"^\s+at ").unwrap(),           // JavaScript/TypeScript
        Regex::new(r"^\s+File .*, line \d+").unwrap(), // Python
        Regex::new(r"^\s+\d+: ").unwrap(),          // Rust
        Regex::new(r"^\s*goroutine \d+").unwrap(),  // Go
        Regex::new(r"^\s*thread").unwrap(),         // General
        Regex::new(r"Traceback \(most recent call last\)").unwrap(), // Python traceback header
    ];

    for line in lines {
        let is_trace_line = trace_patterns.iter().any(|pattern| pattern.is_match(line));

        if is_trace_line {
            if !in_trace {
                in_trace = true;
            }
            current_trace.push(line.to_string());
        } else if in_trace && line.trim().is_empty() {
            // Empty line ends the trace
            if !current_trace.is_empty() {
                traces.push(current_trace.join("\n"));
                current_trace.clear();
            }
            in_trace = false;
        } else if in_trace {
            // Continue collecting trace lines
            current_trace.push(line.to_string());
        }
    }

    // Don't forget the last trace if file ends without empty line
    if !current_trace.is_empty() {
        traces.push(current_trace.join("\n"));
    }

    traces
}

/// Helper to filter out common English words that aren't likely to be code identifiers
fn is_common_word(word: &str) -> bool {
    let common = [
        "the", "this", "that", "with", "from", "have", "been", "were", "will",
        "your", "they", "what", "when", "where", "which", "there", "their",
        "about", "would", "could", "should", "these", "those", "other", "into",
        "than", "then", "them", "some", "only", "over", "also", "just", "like",
        "through", "after", "before", "between", "under", "while", "where",
    ];

    common.contains(&word.to_lowercase().as_str())
}
