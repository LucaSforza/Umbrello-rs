//! `CodeWriter` helper for indentation-aware code generation.

/// Helper for producing indented source code output.
///
/// Manages indentation depth and provides convenience methods for writing
/// lines with proper indentation.
#[derive(Debug, Default)]
pub struct CodeWriter {
    /// The accumulated output text.
    content: String,
    /// Current indentation level (number of indent strings).
    indent_level: usize,
    /// The indent string (e.g., four spaces or tab).
    indent_string: &'static str,
}

impl CodeWriter {
    /// Create a new code writer with the given indent string.
    #[must_use]
    pub fn new(indent_string: &'static str) -> Self {
        Self {
            content: String::new(),
            indent_level: 0,
            indent_string,
        }
    }

    /// Increase indentation level.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level.
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Get the current indentation string.
    #[must_use]
    fn current_indent(&self) -> String {
        self.indent_string.repeat(self.indent_level)
    }

    /// Write a line with current indentation, followed by a newline.
    pub fn writeln(&mut self, line: &str) {
        self.content.push_str(&self.current_indent());
        self.content.push_str(line);
        self.content.push('\n');
    }

    /// Write a blank line.
    pub fn blank_line(&mut self) {
        self.content.push('\n');
    }

    /// Consume the writer and return the generated source code.
    #[must_use]
    pub fn into_string(self) -> String {
        self.content
    }
}
