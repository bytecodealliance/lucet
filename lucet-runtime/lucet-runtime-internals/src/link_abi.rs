lazy_static::lazy_static! {
    pub static ref ENSURE_LINKED: Vec<(String,u64)> = {
        inventory::iter::<LinkAbi>.into_iter()
            .map(|l| (l.name.to_owned(), l.f as u64))
            .collect()
    };
}

pub use inventory::{self, submit};

pub struct LinkAbi {
    name: &'static str,
    f: *const extern "C" fn(),
}

impl LinkAbi {
    pub fn new(name: &'static str, f: *const extern "C" fn()) -> Self {
        Self { name, f }
    }
}

inventory::collect!(LinkAbi);
