use crate::error::Error;
use crate::module::DataInitializer;
use lucet_module::owned::OwnedSparseData;
use lucet_module::HeapSpec;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub use lucet_module::SparseData;

const PAGE_SIZE: u64 = 4096;

fn linear_memory_range<'a>(di: &DataInitializer<'a>, start: u32, end: u32) -> &'a [u8] {
    let offs = di.offset as u32;
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

// If the data initializer contains indexes that are out of range for a 32 bit linear
// memory, this function will return Err(TryFromIntError)
fn split<'a>(di: &DataInitializer<'a>) -> Result<Vec<(u32, DataInitializer<'a>)>, anyhow::Error> {
    use std::convert::TryInto;
    // Divide a data initializer for linear memory into a set of data initializers for pages, and
    // the index of the page they cover.
    // The input initializer can cover many pages. Each output initializer covers exactly one.
    let start = di.offset as u64;
    let end = start
        .checked_add(di.data.len() as u64)
        .ok_or_else(|| anyhow::format_err!("overflow"))?;
    let mut offs = start;
    let mut out = Vec::new();

    while offs < end {
        let page = offs / PAGE_SIZE;
        let page_offs = offs % PAGE_SIZE;
        let next = u64::min((page + 1) * PAGE_SIZE, end);

        let subslice = linear_memory_range(di, offs.try_into()?, next.try_into()?);
        out.push((
            page.try_into()?,
            DataInitializer {
                base: None,
                offset: page_offs.try_into()?,
                data: subslice,
            },
        ));
        offs = next;
    }
    Ok(out)
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
        let chunks = split(initializer).map_err(|_| Error::InitData)?;
        for (pagenumber, chunk) in chunks {
            if pagenumber as u64 > heap.initial_size / PAGE_SIZE {
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
