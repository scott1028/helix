//! TDD tests: control chars are drawn as visible symbols instead of reaching the terminal.

use helix_tui::buffer::Buffer;
use helix_view::graphics::{Rect, Style};

fn drawn_symbol(grapheme: &str, width: usize) -> String {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    buffer.set_grapheme(0, 0, grapheme, width, Style::default());
    buffer[(0, 0)].symbol.to_string()
}

#[test]
fn control_chars_are_drawn_as_pictures() {
    assert_eq!(drawn_symbol("\x1b", 1), "␛");
    assert_eq!(drawn_symbol("\0", 1), "␀");
    assert_eq!(drawn_symbol("\x08", 1), "␈");
    assert_eq!(drawn_symbol("\x7f", 1), "␡");
    assert_eq!(drawn_symbol("\u{9b}", 1), "\u{FFFD}");
}

#[test]
fn other_graphemes_are_unchanged() {
    assert_eq!(drawn_symbol("a", 1), "a");
    assert_eq!(drawn_symbol("漢", 2), "漢");
}
