use std::{ops::Range, path::PathBuf};

use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::{document::SCRATCH_BUFFER_NAME, graphics::Rect, DocumentId, Editor};
use tui::buffer::Buffer as Surface;

/// Tab bar state kept across renders so the scroll position stays stable.
#[derive(Default)]
pub struct Bufferline {
    /// Index of the first visible tab.
    first: usize,
    /// Area of the last render; empty when not rendered.
    area: Rect,
    /// Column range of each visible tab from the last render.
    tabs: Vec<(Range<u16>, DocumentId)>,
}

impl Bufferline {
    pub fn render(&mut self, editor: &Editor, viewport: Rect, surface: &mut Surface) {
        let scratch = PathBuf::from(SCRATCH_BUFFER_NAME); // default filename to use for scratch buffer
        surface.clear_with(
            viewport,
            editor
                .theme
                .try_get("ui.bufferline.background")
                .unwrap_or_else(|| editor.theme.get("ui.statusline")),
        );

        let bufferline_active = editor
            .theme
            .try_get("ui.bufferline.active")
            .unwrap_or_else(|| editor.theme.get("ui.statusline.active"));

        let bufferline_inactive = editor
            .theme
            .try_get("ui.bufferline")
            .unwrap_or_else(|| editor.theme.get("ui.statusline.inactive"));

        let current_doc = view!(editor).doc;
        let tabs: Vec<(DocumentId, String)> = editor
            .documents()
            .map(|doc| {
                let fname = doc
                    .path()
                    .unwrap_or(&scratch)
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default();
                let text = format!(" {}{} ", fname, if doc.is_modified() { "[+]" } else { "" });
                (doc.id(), text)
            })
            .collect();
        let widths: Vec<u16> = tabs
            .iter()
            .map(|(_, text)| u16::try_from(text.width()).unwrap_or(u16::MAX))
            .collect();
        let active = tabs
            .iter()
            .position(|(id, _)| *id == current_doc)
            .unwrap_or(0);
        self.first = first_visible_tab(&widths, active, viewport.width, self.first);
        self.area = viewport;
        self.tabs.clear();

        let mut x = viewport.x;
        for (id, text) in &tabs[self.first..] {
            if x >= viewport.right() {
                break;
            }
            let style = if *id == current_doc {
                bufferline_active
            } else {
                bufferline_inactive
            };
            let rem_width = viewport.right() - x;
            let start = x;
            x = surface
                .set_stringn(x, viewport.y, text, rem_width as usize, style)
                .0;
            self.tabs.push((start..x, *id));
        }
    }

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn tab_at(&self, column: u16, row: u16) -> Option<DocumentId> {
        if row != self.area.y {
            return None;
        }
        self.tabs
            .iter()
            .find(|(range, _)| range.contains(&column))
            .map(|(_, id)| *id)
    }
}

/// Returns the first tab to draw so that the `active` tab is visible.
///
/// Keeps `prev_first` when possible so the tabs don't shift on every switch,
/// and scrolls back left when there is unused space on the right.
pub fn first_visible_tab(widths: &[u16], active: usize, avail: u16, prev_first: usize) -> usize {
    if active >= widths.len() {
        return 0;
    }
    let min_first = fit_start(widths, active, avail);
    let fill_first = fit_start(widths, widths.len() - 1, avail);
    prev_first
        .clamp(min_first, active)
        .min(fill_first.max(min_first))
}

/// Leftmost index such that tabs `start..=end` fit in `avail`; `end` itself always counts.
fn fit_start(widths: &[u16], end: usize, avail: u16) -> usize {
    let mut used = u32::from(widths[end]);
    let mut start = end;
    while start > 0 {
        let next = used + u32::from(widths[start - 1]);
        if next > u32::from(avail) {
            break;
        }
        used = next;
        start -= 1;
    }
    start
}

#[cfg(test)]
mod tests {
    use super::first_visible_tab;

    #[test]
    fn all_tabs_fit() {
        assert_eq!(first_visible_tab(&[3, 3, 3], 2, 20, 0), 0);
    }

    #[test]
    fn empty_tab_list() {
        assert_eq!(first_visible_tab(&[], 0, 20, 3), 0);
    }

    #[test]
    fn active_at_end_scrolls_right() {
        assert_eq!(first_visible_tab(&[5; 5], 4, 12, 0), 3);
    }

    #[test]
    fn active_at_start_scrolls_left() {
        assert_eq!(first_visible_tab(&[5; 5], 0, 12, 3), 0);
    }

    #[test]
    fn keeps_position_while_active_visible() {
        assert_eq!(first_visible_tab(&[5; 5], 3, 12, 2), 2);
    }

    #[test]
    fn single_tab_wider_than_viewport() {
        assert_eq!(first_visible_tab(&[30], 0, 10, 0), 0);
        assert_eq!(first_visible_tab(&[5, 30, 5], 1, 10, 0), 1);
    }

    #[test]
    fn scrolls_back_when_space_frees_up() {
        assert_eq!(first_visible_tab(&[5, 5, 5], 2, 20, 2), 0);
    }
}
