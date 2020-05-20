mod c_child;
mod rust_child;
use crate::context::{Context, ContextHandle, Error, InstanceExitData};
use memoffset::offset_of;
use std::slice;

#[test]
fn context_offsets_correct() {
    assert_eq!(offset_of!(Context, gpr), 0);
    assert_eq!(offset_of!(Context, fpr), 10 * 8);
    assert_eq!(offset_of!(Context, exit_data), 10 * 8 + 8 * 16);

    let exit_data_offset = offset_of!(Context, exit_data);
}

#[test]
fn exit_data_offsets_correct() {
    assert_eq!(offset_of!(InstanceExitData, retvals_gp), 0);
    assert_eq!(offset_of!(InstanceExitData, retval_fp), 8 * 2);
    assert_eq!(offset_of!(InstanceExitData, parent_ctx), 8 * 2 + 16);
    assert_eq!(
        offset_of!(InstanceExitData, backstop_callback),
        8 * 2 + 16 + 8
    );
    assert_eq!(
        offset_of!(InstanceExitData, callback_data),
        8 * 2 + 16 + 8 + 8
    );
}

#[test]
fn init_rejects_unaligned() {
    extern "C" fn dummy() {}
    // first we have to specially craft an unaligned slice, since
    // a normal allocation of a [u64] often ends up 16-byte
    // aligned
    let mut len = 1024;
    let mut stack = vec![0u64; len];
    let ptr = stack.as_mut_ptr();
    let skew = ptr as usize % 16;

    // we happened to be aligned already, so let's mess it up
    if skew == 0 {
        len -= 1;
    }

    let mut stack_unaligned = unsafe { slice::from_raw_parts_mut(ptr, len) };

    // now we have the unaligned stack, let's make sure it blows up right
    let res = ContextHandle::create_and_init(&mut stack_unaligned, dummy as usize, &[]);

    if let Err(Error::UnalignedStack) = res {
        assert!(true);
    } else {
        assert!(false, "init succeeded with unaligned stack");
    }
}
