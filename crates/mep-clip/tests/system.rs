//! Manual round trip through the real system clipboard (IMPLEMENTATION_PLAN.md §9.4)
//!
//! Run on a desktop session with `cargo test -p mep-clip -- --ignored`.

#![feature(coverage_attribute)]
#![coverage(off)]

use mep_clip::{Adapter, system::ClipboardRs};
use mep_core::clip::{Clip, Shape, SystemClipboard};

#[test]
#[ignore = "needs a desktop session with a system clipboard"]
fn box_round_trip_through_the_system_clipboard()
{
    let mut clipboard = Adapter::new(ClipboardRs::new().expect("system clipboard"));
    let clip = Clip { shape: Shape::Box { width: 3 }, lines: vec![b"ab ".to_vec(), b"cde".to_vec()] };
    assert!(clipboard.write(&clip.to_payload("\n")));
    let read = clipboard.read().expect("clipboard text");
    assert_eq!(Clip::from_payload(&read).shape, Shape::Box { width: 3 });
}
