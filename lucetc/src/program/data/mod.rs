pub mod sparse;

use super::init_expr::const_init_expr;
use failure::{format_err, Error, ResultExt};
use parity_wasm::elements::Module;

#[derive(Debug)]
pub struct DataInit<'m> {
    pub offset: u32,
    pub data: &'m [u8],
}

impl<'m> DataInit<'m> {
    pub fn linear_memory_range(&self, start: usize, end: usize) -> &'m [u8] {
        let offs = self.offset as usize;
        assert!(end >= start);
        assert!(start >= offs);
        assert!(start < offs + self.data.len());
        assert!(end >= offs);
        assert!(end <= offs + self.data.len());
        &self.data[(start - offs)..(end - offs)]
    }
}

pub fn module_data<'m>(module: &'m Module) -> Result<Vec<DataInit<'m>>, Error> {
    let mut initializers = Vec::new();
    // XXX check the location of these init sections against the size of the memory imported or
    // declared. That means we need to actually care about memories that we see in module.rs
    if let Some(data_section) = module.data_section() {
        for (segment_ix, segment) in data_section.entries().iter().enumerate() {
            if segment.index() != 0 {
                // https://webassembly.github.io/spec/syntax/modules.html#data-segments
                return Err(format_err!(
                    "In the current version of WebAssembly, at most one memory is \
                     allowed in a module. Consequently, the only valid memidx is 0 \
                     (segment index={})",
                    segment.index(),
                ));
            }

            // Take the offset, and treat it as an unsigned 32 bit number.
            // XXX need a type checked const_init_expr - this should always be a u32.
            let offset = const_init_expr(
                segment
                    .offset()
                    .as_ref()
                    .ok_or(format_err!("Offset not found"))?
                    .code(),
            )
            .context(format!("data segment {}", segment_ix))? as u32;

            let max_lm_size: i64 = 0xFFFFFFFF; // 4GiB, per spec

            // Compare them at i64 so that they do not overflow
            if (offset as i64) + (segment.value().len() as i64) > max_lm_size {
                return Err(format_err!(
                    "initalizer does not fit in linear memory offset={} len={}",
                    offset,
                    segment.value().len()
                ));
            }

            initializers.push(DataInit {
                offset: offset,
                data: segment.value(),
            })
        }
    }
    Ok(initializers)
}
