use super::error::IDLError;
use std::cell::RefCell;
use std::convert::Into;
use std::io::prelude::*;
use std::rc::Rc;

/// Write indented code
/// #[derive(Clone)]
pub struct PrettyWriter<W: Write> {
    writer: Rc<RefCell<W>>,
    indent: u32,
    indent_bytes: Vec<u8>,
}

impl<W: Write> PrettyWriter<W> {
    /// Create a new `PrettyWriter` with `indent` initial units of indentation
    pub fn new_with_indent(writer: W, indent: u32) -> Self {
        PrettyWriter {
            writer: Rc::new(RefCell::new(writer)),
            indent,
            indent_bytes: b"    ".to_vec(),
        }
    }

    /// Create a new `PrettyWriter` with no initial indentation
    pub fn new(writer: W) -> Self {
        PrettyWriter::new_with_indent(writer, 0)
    }

    /// Create a writer based on a existing writer, but with no indentation`
    pub fn new_from_writer(&mut self) -> Self {
        PrettyWriter {
            writer: self.writer.clone(),
            indent: 0,
            indent_bytes: self.indent_bytes.clone(),
        }
    }

    /// Create an indented block within the current `PrettyWriter`
    pub fn new_block(&mut self) -> Self {
        PrettyWriter {
            writer: self.writer.clone(),
            indent: self.indent + 1,
            indent_bytes: self.indent_bytes.clone(),
        }
    }

    fn _write_all(writer: &mut W, buf: &[u8]) -> Result<(), IDLError> {
        writer.write_all(buf).map_err(Into::into)
    }

    /// Return the current indentation level
    #[allow(dead_code)]
    pub fn indent_level(&self) -> u32 {
        self.indent
    }

    /// Output an indentation string
    pub fn indent(&mut self) -> Result<&mut Self, IDLError> {
        let indent_bytes = &self.indent_bytes.clone();
        {
            let mut writer = self.writer.borrow_mut();
            for _ in 0..self.indent {
                Self::_write_all(&mut writer, indent_bytes)?
            }
        }
        Ok(self)
    }

    /// Output a space
    pub fn space(&mut self) -> Result<&mut Self, IDLError> {
        Self::_write_all(&mut self.writer.borrow_mut(), b" ")?;
        Ok(self)
    }

    /// Output an end of line
    pub fn eol(&mut self) -> Result<&mut Self, IDLError> {
        Self::_write_all(&mut self.writer.borrow_mut(), b"\n")?;
        Ok(self)
    }

    /// Output a block separator
    pub fn eob(&mut self) -> Result<&mut Self, IDLError> {
        self.eol()
    }

    /// Continuation
    pub fn continuation(&mut self) -> Result<&mut Self, IDLError> {
        self.indent()?;
        let indent_bytes = &self.indent_bytes.clone();
        Self::_write_all(&mut self.writer.borrow_mut(), indent_bytes)?;
        Ok(self)
    }

    /// Write raw data
    pub fn write(&mut self, buf: &[u8]) -> Result<&mut Self, IDLError> {
        Self::_write_all(&mut self.writer.borrow_mut(), buf)?;
        Ok(self)
    }

    /// Indent, write raw data and terminate with an end of line
    pub fn write_line(&mut self, buf: &[u8]) -> Result<&mut Self, IDLError> {
        self.indent()?.write(buf)?.eol()
    }

    pub fn into_inner(self) -> Option<W> {
        Rc::try_unwrap(self.writer).ok().map(|w| w.into_inner())
    }
}
