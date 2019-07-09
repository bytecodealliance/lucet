#[repr(C)]
#[derive(Clone, Debug)]
pub struct TableElement {
    ty: u64,
    pub rf: u64,
}
