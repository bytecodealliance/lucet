use nix::unistd::{sysconf, SysconfVar};
use std::sync::Once;

pub const HOST_PAGE_SIZE_EXPECTED: usize = 4096;
static mut HOST_PAGE_SIZE: usize = 0;
static HOST_PAGE_SIZE_INIT: Once = Once::new();

/// Linux x86-64 and Mac x86-64 hosts should always use a 4K page.
///
/// We double check the expected value using `sysconf` at runtime.
pub fn host_page_size() -> usize {
    unsafe {
        HOST_PAGE_SIZE_INIT.call_once(|| match sysconf(SysconfVar::PAGE_SIZE) {
            Ok(Some(sz)) => {
                if sz as usize == HOST_PAGE_SIZE_EXPECTED {
                    HOST_PAGE_SIZE = HOST_PAGE_SIZE_EXPECTED;
                } else {
                    panic!(
                        "host page size was {}; expected {}",
                        sz, HOST_PAGE_SIZE_EXPECTED
                    );
                }
            }
            _ => panic!("could not get host page size from sysconf"),
        });
        HOST_PAGE_SIZE
    }
}
