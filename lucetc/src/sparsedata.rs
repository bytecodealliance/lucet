use crate::error::Error;
use crate::module::DataInitializer;
use lucet_module::owned::OwnedSparseData;
use lucet_module::HeapSpec;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub use lucet_module::SparseData;

const PAGE_SIZE: usize = 4096;

fn linear_memory_range<'a>(di: &DataInitializer<'a>, start: usize, end: usize) -> &'a [u8] {
    let offs = di.offset as usize;
    // The range of linear memory we're interested in is:
    // valid: end is past the start
    assert!(end >= start);
    // in this initializer: starts at or past the offset of this initializer,
    assert!(start >= offs);
    // and before the end of this initializer,
    assert!(start < offs + di.data.len());
    // ends past the offset of this initializer (redundant: implied by end >= start),
    assert!(end >= offs);
    // and ends before or at the end of this initializer.
    assert!(end <= offs + di.data.len());
    &di.data[(start - offs)..(end - offs)]
}

fn split<'a>(di: &DataInitializer<'a>) -> Vec<(usize, DataInitializer<'a>)> {
    // Divide a data initializer for linear memory into a set of data initializers for pages, and
    // the index of the page they cover.
    // The input initializer can cover many pages. Each output initializer covers exactly one.
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

pub fn owned_sparse_data_from_initializers<'a>(
    initializers: &[DataInitializer<'a>],
    heap: &HeapSpec,
) -> Result<OwnedSparseData, Error> {
    let mut pagemap: HashMap<usize, Vec<u8>> = HashMap::new();

    for initializer in initializers {
        if initializer.base.is_some() {
            let message =
                "cannot create sparse data: data initializer uses global as base".to_owned();
            Err(Error::Unsupported(message))?;
        }
        let chunks = split(initializer);
        for (pagenumber, chunk) in chunks {
            if pagenumber > heap.initial_size as usize / PAGE_SIZE {
                Err(Error::InitData)?;
            }
            let base = chunk.offset as usize;
            let page = match pagemap.entry(pagenumber) {
                Entry::Occupied(occ) => occ.into_mut(),
                Entry::Vacant(vac) => vac.insert(vec![0; PAGE_SIZE]),
            };
            page[base..base + chunk.data.len()].copy_from_slice(chunk.data);
            debug_assert!(page.len() == PAGE_SIZE);
        }
    }

    assert_eq!(heap.initial_size as usize % PAGE_SIZE, 0);
    let highest_page = heap.initial_size as usize / PAGE_SIZE;

    let mut out = Vec::new();
    for page_ix in 0..highest_page {
        if let Some(chunk) = pagemap.remove(&page_ix) {
            assert!(chunk.len() == 4096);
            out.push(Some(chunk))
        } else {
            out.push(None)
        }
    }
    assert_eq!(out.len() * 4096, heap.initial_size as usize);
    let o = OwnedSparseData::new(out)?;
    Ok(o)
}
