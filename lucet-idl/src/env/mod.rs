pub mod atoms;
pub mod cursor;
pub mod prelude;
pub mod repr;
pub mod validate;

pub trait MemArea {
    fn mem_size(&self) -> usize;
    fn mem_align(&self) -> usize;
}
