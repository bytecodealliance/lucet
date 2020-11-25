/// This struct describes the handful of fields that Lucet-compiled programs may directly interact with, but
/// are provided through VMContext.
#[repr(C)]
#[repr(align(8))]
pub struct InstanceRuntimeData {
    pub globals_ptr: *mut i64,
    /// `instruction_count_bound + instruction_count_adj` gives the total
    /// instructions executed. We deconstruct the count into a signed adjustment
    /// and a "bound" because we want to be able to set a runtime bound beyond
    /// which we yield to the caller. We do this by beginning execution with
    /// `instruction_count_adj` set to some negative value and
    /// `instruction_count_bound` adjusted upward in compensation.
    /// `instruction_count_adj` is incremented as execution proceeds; on each
    /// increment, the Wasm code checks the sign. If the value is greater than
    /// zero, then we have exceeded the bound and we must yield. At any point,
    /// the `adj` value can be adjusted downward by transferring the count to
    /// the `bound`.
    pub instruction_count_adj: i64,
    pub instruction_count_bound: i64,
    pub stack_limit: u64,
}
