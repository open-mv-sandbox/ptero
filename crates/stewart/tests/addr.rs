use std::mem::size_of;

use stewart::handler::SenderT;

#[test]
fn system_addr_option_same_size() {
    // This should be provided to us by the underlying Index type from thunderdome
    // But, it's good to verify just in case
    let size_plain = size_of::<SenderT<()>>();
    let size_option = size_of::<Option<SenderT<()>>>();
    assert_eq!(size_plain, size_option);
}
