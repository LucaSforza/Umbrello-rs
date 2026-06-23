//! CodeWriter helper for indentation-aware code generation.

/// Indentation-aware code writer.
pub struct CodeWriter {
    content: String,
    indent_level: usize,
    indent_string: &'static str,
}

impl CodeWriter {
    /// Create a new `CodeWriter` with the given indent string.
    #[must_use]
    pub fn new(indent_string: &'static str) -> Self {
        Self {
            content: String::new(),
            indent_level: 0,
            indent_string,
        }
    }

    /// Increase the indent level by one.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease the indent level by one (no-op if at zero).
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Return the current indent string.
    fn current_indent(&self) -> String {
        self.indent_string.repeat(self.indent_level)
    }

    /// Write a line (indentation + content + newline).
    pub fn writeln(&mut self, line: &str) {
        self.content.push_str(&self.current_indent());
        self.content.push_str(line);
        self.content.push('\n');
    }

    /// Write a blank line.
    pub fn blank_line(&mut self) {
        self.content.push('\n');
    }

    /// Consume the writer and return the accumulated string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.content
    }
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::new("    ")
    }
}
