pub mod atoms;
pub mod cursor;
pub mod datatypes;
pub mod function;
pub mod prelude;
pub mod repr;

pub trait MemArea {
    fn mem_size(&self) -> usize;
    fn mem_align(&self) -> usize;
}
