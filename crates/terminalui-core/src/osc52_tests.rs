//! The OSC 52 writer: known-answer base64 and clipboard payload framing.

use super::{base64, write};

#[test]
fn base64_matches_the_reference_vectors() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"M"), "TQ==");
    assert_eq!(base64(b"Ma"), "TWE=");
    assert_eq!(base64(b"Man"), "TWFu");
    assert_eq!(
        base64(b"any carnal pleasure."),
        "YW55IGNhcm5hbCBwbGVhc3VyZS4="
    );
    assert_eq!(base64(&[0xff, 0xee, 0xdd, 0xdc, 0xcb, 0xba]), "/+7d3Mu6");
}

#[test]
fn write_outside_a_terminal_is_a_noop() {
    assert!(write("clipboard").is_ok());
}
