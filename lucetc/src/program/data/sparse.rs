use super::DataInit;
use crate::program::memory::HeapSpec;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub use lucet_module_data::writer::SparseData;

const PAGE_SIZE: usize = 4096;

fn split<'m>(di: &DataInit<'m>) -> Vec<(usize, DataInit<'m>)> {
    let start = di.offset as usize;
    let end = start + di.data.len();
    let mut offs = start;
    let mut out = Vec::new();

    while offs < end {
        let page = offs / PAGE_SIZE;
        let page_offs = offs % PAGE_SIZE;
        let next = usize::min((page + 1) * PAGE_SIZE, end);

        let subslice = di.linear_memory_range(offs, next);
        out.push((
            page,
            DataInit {
                offset: page_offs as u32,
                data: subslice,
            },
        ));
        offs = next;
    }
    out
}

pub fn make_sparse<'m>(data: &[DataInit<'m>], heap: HeapSpec) -> SparseData {
    let mut m: HashMap<usize, Vec<u8>> = HashMap::new();

    for d in data {
        let chunks = split(d);
        for (pagenumber, chunk) in chunks {
            let base = chunk.offset as usize;
            match m.entry(pagenumber) {
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

    assert_eq!(heap.initial_size as usize % PAGE_SIZE, 0);
    let highest_page = heap.initial_size as usize / PAGE_SIZE;

    let mut out = Vec::new();
    for page_ix in 0..highest_page {
        if let Some(chunk) = m.remove(&page_ix) {
            assert!(chunk.len() == 4096);
            out.push(Some(chunk))
        } else {
            out.push(None)
        }
    }
    assert_eq!(out.len() * 4096, heap.initial_size as usize);
    SparseData::new(out).expect("sparse data invariants held")
}
