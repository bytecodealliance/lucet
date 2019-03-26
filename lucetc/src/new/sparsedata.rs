use failure::{Error, format_err};
use crate::program::memory::HeapSpec;
use crate::new::module::DataInitializer;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub use lucet_module_data::SparseData;

const PAGE_SIZE: usize = 4096;

fn linear_memory_range<'a>(di: &DataInitializer<'a>, start: usize, end: usize) -> &'a [u8] {
        let offs = di.offset as usize;
        assert!(end >= start);
        assert!(start >= offs);
        assert!(start < offs + di.data.len());
        assert!(end >= offs);
        assert!(end <= offs + di.data.len());
        &di.data[(start - offs)..(end - offs)]
}

fn split<'a>(di: &DataInitializer<'a>) -> Vec<(usize, DataInitializer<'a>)> {
    let start = di.offset as usize;
    let end = start + di.data.len();
    let mut offs = start;
    let mut out = Vec::new();

    while offs < end {
        let page = offs / PAGE_SIZE;
        let page_offs = offs % PAGE_SIZE;
        let next = usize::min((page + 1) * PAGE_SIZE, end);

        let subslice = linear_memory_range(di, offs, next);
        out.push((
            page,
            DataInitializer {
                base: None,
                offset: page_offs,
                data: subslice,
            },
        ));
        offs = next;
    }
    out
}

pub struct OwnedSparseData {
    pagemap: HashMap<usize, Vec<u8>>,
    heap: HeapSpec,
}

impl OwnedSparseData {
    pub fn new<'a>(initializers: &[DataInitializer<'a>], heap: HeapSpec) -> Result<Self, Error> {
        let mut pagemap: HashMap<usize, Vec<u8>> = HashMap::new();

        for initializer in initializers {
            if initializer.base.is_some() {
                Err(format_err!("cannot create sparse data: data initializer uses global as base"))?
            }
            let chunks = split(initializer);
            for (pagenumber, chunk) in chunks {
                let base = chunk.offset as usize;
                match pagemap.entry(pagenumber) {
                    Entry::Occupied(occ) => {
                        let occ = occ.into_mut();
                        for (offs, data) in chunk.data.iter().enumerate() {
                            occ[base + offs] = *data;
                        }
                        assert!(occ.len() == PAGE_SIZE);
                    }
                    Entry::Vacant(vac) => {
                        let vac = vac.insert(Vec::new());
                        vac.resize(PAGE_SIZE, 0);
                        for (offs, data) in chunk.data.iter().enumerate() {
                            vac[base + offs] = *data;
                        }
                        assert!(vac.len() == PAGE_SIZE);
                    }
                }
            }
        }
        Ok(Self { pagemap, heap })
    }

    pub fn sparse_data(&self) -> SparseData {
        assert_eq!(self.heap.initial_size as usize % PAGE_SIZE, 0);
        let highest_page = self.heap.initial_size as usize / PAGE_SIZE;

        let mut out = Vec::new();
        for page_ix in 0..highest_page {
            if let Some(chunk) = self.pagemap.get(&page_ix) {
                assert!(chunk.len() == 4096);
                out.push(Some(chunk.as_slice()))
            } else {
                out.push(None)
            }
        }
        assert_eq!(out.len() * 4096, self.heap.initial_size as usize);
        SparseData::new(out).expect("sparse data invariants held")
    }
}
