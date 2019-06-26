use super::error::IDLError;
use std::cell::RefCell;
use std::convert::Into;
use std::io::prelude::*;
use std::rc::Rc;

/// Write indented code
/// #[derive(Clone)]
pub struct PrettyWriter {
    writer: Rc<RefCell<Box<dyn Write>>>,
    indent: u32,
    indent_bytes: Vec<u8>,
}

impl PrettyWriter {
    /// Create a new `PrettyWriter` with `indent` initial units of indentation
    pub fn new_with_indent(writer: Box<dyn Write>, indent: u32) -> Self {
        PrettyWriter {
            writer: Rc::new(RefCell::new(writer)),
            indent,
            indent_bytes: b"    ".to_vec(),
        }
    }

    /// Create a new `PrettyWriter` with no initial indentation
    pub fn new(writer: Box<dyn Write>) -> Self {
        PrettyWriter::new_with_indent(writer, 0)
    }

    /// Create an indented block within the current `PrettyWriter`
    pub fn new_block(&mut self) -> Self {
        PrettyWriter {
            writer: self.writer.clone(),
            indent: self.indent + 1,
            indent_bytes: self.indent_bytes.clone(),
        }
    }

    pub fn indent(&mut self) -> &mut Self {
        self.indent += 1;
        self
    }

    fn _write_all(&mut self, buf: &[u8]) -> Result<(), IDLError> {
        self.writer.borrow_mut().write_all(buf).map_err(Into::into)
    }

    /// Return the current indentation level
    #[allow(dead_code)]
    pub fn indent_level(&self) -> u32 {
        self.indent
    }

    /// Output an indentation string
    fn write_indent(&mut self) -> Result<&mut Self, IDLError> {
        let indent_bytes = &self.indent_bytes.clone();
        {
            for _ in 0..self.indent {
                self._write_all(indent_bytes)?
            }
        }
        Ok(self)
    }

    /// Output an end of line
    pub fn eol(&mut self) -> Result<&mut Self, IDLError> {
        self._write_all(b"\n")?;
        Ok(self)
    }

    /// Output a block separator
    pub fn eob(&mut self) -> Result<&mut Self, IDLError> {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.eol()
    }

    /// Write raw data
    pub fn write(&mut self, buf: &[u8]) -> Result<&mut Self, IDLError> {
        self._write_all(buf)?;
        Ok(self)
    }

    /// Indent, write raw data and terminate with an end of line
    pub fn write_line(&mut self, buf: &[u8]) -> Result<&mut Self, IDLError> {
        self.write_indent()?.write(buf)?.eol()
    }

    /// Indent, write raw data and terminate with an end of line
    pub fn writeln<S: AsRef<str>>(&mut self, buf: S) -> Result<&mut Self, IDLError> {
        self.write_line(buf.as_ref().as_bytes())
    }
}
