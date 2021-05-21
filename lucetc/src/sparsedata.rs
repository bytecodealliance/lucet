use crate::error::Error;
use crate::module::DataInitializer;
use lucet_module::owned::OwnedSparseData;
use lucet_module::HeapSpec;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub use lucet_module::SparseData;

const PAGE_SIZE: u32 = 4096;

fn linear_memory_range<'a>(di: &DataInitializer<'a>, start: u32, end: u32) -> &'a [u8] {
    let offs = di.offset;
    // The range of linear memory we're interested in is:
    // valid: end is past the start
    assert!(end >= start);
    // in this initializer: starts at or past the offset of this initializer,
    assert!(start >= offs);
    // and before the end of this initializer,
    assert!(start < offs.checked_add(di.data.len() as u32).unwrap());
    // ends past the offset of this initializer (redundant: implied by end >= start),
    assert!(end >= offs);
    // and ends before or at the end of this initializer.
    assert!(end <= offs.checked_add(di.data.len() as u32).unwrap());
    &di.data[((start - offs) as usize)..((end - offs) as usize)]
}

// XXX when changing the start/end/offsets from usize to u32 i introduced a bug here! fix it on
// monday
fn split<'a>(di: &DataInitializer<'a>) -> Vec<(u32, DataInitializer<'a>)> {
    // Divide a data initializer for linear memory into a set of data initializers for pages, and
    // the index of the page they cover.
    // The input initializer can cover many pages. Each output initializer covers exactly one.
    let start = di.offset;
    let end = start + di.data.len() as u32;
    let mut offs = start;
    let mut out = Vec::new();

    while offs < end {
        let page = offs / PAGE_SIZE;
        let page_offs = offs % PAGE_SIZE;
        let next = u32::min((page + 1) * PAGE_SIZE, end);

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
    let mut pagemap: HashMap<u32, Vec<u8>> = HashMap::new();

    for initializer in initializers {
        if initializer.base.is_some() {
            let message =
                "cannot create sparse data: data initializer uses global as base".to_owned();
            return Err(Error::Unsupported(message));
        }
        let chunks = split(initializer);
        for (pagenumber, chunk) in chunks {
            if pagenumber > heap.initial_size as u32 / PAGE_SIZE {
                return Err(Error::InitData);
            }
            let base = chunk.offset as usize;
            let page = match pagemap.entry(pagenumber) {
                Entry::Occupied(occ) => occ.into_mut(),
                Entry::Vacant(vac) => vac.insert(vec![0; PAGE_SIZE as usize]),
            };
            page[base..base + chunk.data.len()].copy_from_slice(chunk.data);
            debug_assert!(page.len() == PAGE_SIZE as usize);
        }
    }

    assert_eq!(heap.initial_size % PAGE_SIZE as u64, 0);
    let highest_page = heap.initial_size / PAGE_SIZE as u64;

    let mut out = Vec::new();
    for page_ix in 0..highest_page {
        if let Some(chunk) = pagemap.remove(&(page_ix as u32)) {
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
