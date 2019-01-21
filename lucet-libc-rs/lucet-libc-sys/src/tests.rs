use super::*;

#[test]
fn create_lucet_libc() {
    let mut libc = lucet_libc {
        magic: 0,
        term_info: lucet_libc__bindgen_ty_1 { exit: 0 },
        term_reason: lucet_libc_term_reason_lucet_libc_term_none,
        stdio_handler: None,
    };

    unsafe { lucet_libc_init(&mut libc as *mut lucet_libc) };
    assert_eq!(false, unsafe {
        lucet_libc_terminated(&libc as *const lucet_libc)
    });
    assert_eq!(false, unsafe {
        lucet_libc_aborted(&libc as *const lucet_libc)
    });
    assert_eq!(false, unsafe {
        lucet_libc_exited(&libc as *const lucet_libc)
    });
}
