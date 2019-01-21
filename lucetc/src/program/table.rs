use failure::{format_err, Error};
use std::cmp;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TableBuilder {
    index: u32,
    min_size: u32,
    max_size: Option<u32>,
    /// Map from icall index to function
    elems: HashMap<usize, u32>,
}

impl TableBuilder {
    pub fn new(index: u32, min_size: u32, max_size: Option<u32>) -> Result<Self, Error> {
        if let Some(max_size) = max_size {
            if min_size > max_size {
                return Err(format_err!(
                    "table size max ({}) less than min ({})",
                    max_size,
                    min_size
                ));
            }
        }
        Ok(Self {
            index: index,
            min_size: min_size,
            max_size: max_size,
            elems: HashMap::new(),
        })
    }

    pub fn push_elements(&mut self, offset: i32, elems: Vec<u32>) -> Result<(), Error> {
        if offset < 0 {
            return Err(format_err!(
                "table elements given at negative offset {}",
                offset
            ));
        }

        for (i, e) in elems.iter().enumerate() {
            if i > <u32>::max_value() as usize {
                return Err(format_err!("table element at {} out-of-bounds", i));
            }
            // Note: we do not validate that `e` is the index of a valid function. We count on
            // `compiler::table` to check on this.
            if let Some(max_size) = self.max_size {
                if (i as u32) >= max_size {
                    return Err(format_err!(
                        "table element at {} beyond declared maximum size {}",
                        i,
                        max_size
                    ));
                }
            }
            self.elems.insert(offset as usize + i, *e);
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        // Guaranteed by `push_elements` to be <= (max - 1) if there is a max.
        let highest_index = *self.elems.keys().max().unwrap_or(&0);
        // Make table big enough to represent greatest index, or the given minimum size.
        cmp::max(highest_index + 1, self.min_size as usize)
    }

    pub fn finalize(&self) -> TableDef {
        let capacity = self.capacity();
        let mut elems = Vec::with_capacity(capacity);
        for index in 0..capacity {
            match self.elems.get(&index) {
                Some(value) => elems.push(TableElem::FunctionIx(*value)),
                None => elems.push(TableElem::Empty),
            }
        }

        TableDef {
            index: self.index,
            elems: elems,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableElem {
    Empty,
    FunctionIx(u32),
}

#[derive(Debug, Clone)]
pub struct TableDef {
    index: u32,
    elems: Vec<TableElem>,
}

impl TableDef {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn elements(&self) -> &[TableElem] {
        &self.elems
    }
    pub fn symbol(&self) -> String {
        format!("guest_table_{}", self.index)
    }
    pub fn len(&self) -> u64 {
        assert!(self.elems.len() <= ::std::u32::MAX as usize);
        self.elems.len() as u64 * (2 * 8)
    }
    pub fn len_symbol(&self) -> String {
        format!("{}_len", self.symbol())
    }
}
