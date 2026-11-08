use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
};

use helix_core::unicode::width::{UnicodeWidthChar, UnicodeWidthStr};
use helix_view::{
    editor::FileExplorerConfig,
    graphics::{Modifier, Rect},
    input::KeyEvent,
    Theme,
};
use tui::buffer::Buffer as Surface;

use crate::{commands::MappableCommand, ctrl, key, ui::directory_entries};

/// What the caller should do after the tree handled a key.
#[derive(Debug, PartialEq, Eq)]
pub enum TreeAction {
    Ignored,
    Consumed,
    Open(PathBuf),
    Unfocus,
}

#[derive(Debug)]
struct Node {
    path: PathBuf,
    /// Path relative to the parent row; flattened dirs show as `a/b/c`.
    name: String,
    depth: usize,
    is_dir: bool,
    expanded: bool,
}

/// Lazily loaded directory tree, flattened into the rows shown in the sidebar.
pub struct FileTree {
    root: PathBuf,
    nodes: Vec<Node>,
    selected: usize,
    /// First visible row.
    offset: usize,
    /// Number of visible rows, set on render.
    height: usize,
}

impl FileTree {
    pub fn new(root: PathBuf, config: &FileExplorerConfig) -> Self {
        let mut nodes = Vec::new();
        read_tree(&root, 0, &HashSet::new(), config, &mut nodes);
        Self {
            root,
            nodes,
            selected: 0,
            offset: 0,
            height: 0,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn select(&mut self, idx: usize) {
        if idx < self.nodes.len() {
            self.selected = idx;
        }
    }

    pub fn move_by(&mut self, delta: isize) {
        let last = self.nodes.len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    /// Expands or collapses the selected directory, or returns the selected file.
    pub fn activate_selected(&mut self, config: &FileExplorerConfig) -> TreeAction {
        let Some(node) = self.nodes.get(self.selected) else {
            return TreeAction::Consumed;
        };
        if !node.is_dir {
            return TreeAction::Open(node.path.clone());
        }
        if node.expanded {
            self.collapse_selected();
        } else {
            self.expand_selected(config);
        }
        TreeAction::Consumed
    }

    fn expand_selected(&mut self, config: &FileExplorerConfig) {
        let idx = self.selected;
        let Some(node) = self.nodes.get_mut(idx) else {
            return;
        };
        if !node.is_dir || node.expanded {
            return;
        }
        node.expanded = true;
        let mut children = Vec::new();
        read_tree(
            &node.path,
            node.depth + 1,
            &HashSet::new(),
            config,
            &mut children,
        );
        self.nodes.splice(idx + 1..idx + 1, children);
    }

    fn collapse_selected(&mut self) {
        let idx = self.selected;
        let Some(node) = self.nodes.get_mut(idx) else {
            return;
        };
        if !node.expanded {
            return;
        }
        node.expanded = false;
        let depth = node.depth;
        let end = self.nodes[idx + 1..]
            .iter()
            .position(|node| node.depth <= depth)
            .map_or(self.nodes.len(), |pos| idx + 1 + pos);
        self.nodes.drain(idx + 1..end);
    }

    /// Collapses the selected directory, or selects the parent row.
    fn collapse_or_select_parent(&mut self) {
        let Some(node) = self.nodes.get(self.selected) else {
            return;
        };
        if node.expanded {
            self.collapse_selected();
            return;
        }
        let depth = node.depth;
        if let Some(parent) = self.nodes[..self.selected]
            .iter()
            .rposition(|node| node.depth < depth)
        {
            self.selected = parent;
        }
    }

    /// Reloads all rows from disk, keeping expanded dirs and the selected path.
    pub fn refresh(&mut self, config: &FileExplorerConfig) {
        let expanded: HashSet<PathBuf> = self
            .nodes
            .iter()
            .filter(|node| node.expanded)
            .map(|node| node.path.clone())
            .collect();
        let selected_path = self.nodes.get(self.selected).map(|node| node.path.clone());

        let mut nodes = Vec::new();
        read_tree(&self.root, 0, &expanded, config, &mut nodes);
        self.nodes = nodes;

        if let Some(idx) =
            selected_path.and_then(|path| self.nodes.iter().position(|node| node.path == path))
        {
            self.selected = idx;
        }
        self.move_by(0);
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.scroll_by(0);
    }

    /// Scrolls the selected row into view.
    pub fn ensure_visible(&mut self) {
        if self.height == 0 {
            return;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + self.height {
            self.offset = self.selected + 1 - self.height;
        }
    }

    /// Scrolls the view without moving the selection.
    pub fn scroll_by(&mut self, delta: isize) {
        let max = self.nodes.len().saturating_sub(self.height);
        self.offset = self.offset.saturating_add_signed(delta).min(max);
    }

    /// Node index shown at `row`, counted from the first visible row.
    pub fn row_at(&self, row: usize) -> Option<usize> {
        let idx = self.offset + row;
        (idx < self.nodes.len()).then_some(idx)
    }

    fn title(&self) -> Cow<'_, str> {
        self.root.file_name().map_or_else(
            || self.root.to_string_lossy(),
            |name| name.to_string_lossy(),
        )
    }

    /// Columns needed to show the title and every row uncut, border included.
    pub fn content_width(&self) -> u16 {
        let title = 1 + self.title().width();
        // Row text is " " + indent + icon + name, as drawn by `render`.
        let widest = self
            .nodes
            .iter()
            .map(|node| 3 + 2 * node.depth + node.name.width())
            .fold(title, usize::max);
        // One padding column, then the border.
        u16::try_from(widest + 2).unwrap_or(u16::MAX)
    }

    /// Draws a title row, the visible rows and a right border.
    pub fn render(&mut self, area: Rect, surface: &mut Surface, theme: &Theme, is_focused: bool) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let border_x = area.right() - 1;
        let border_style = theme.get("ui.window");
        for y in area.top()..area.bottom() {
            surface.set_string(border_x, y, "│", border_style);
        }

        let width = usize::from(area.width - 1);
        surface.set_stringn(
            area.x,
            area.y,
            &format!(" {}", truncate_middle(&self.title(), width - 1)),
            width,
            theme.get("ui.text").add_modifier(Modifier::BOLD),
        );

        self.set_height(usize::from(area.height - 1));
        let text_style = theme.get("ui.text");
        let directory_style = theme.get("ui.text.directory");
        let selected_style = theme.get("ui.menu.selected");
        let rows = self.nodes.iter().enumerate().skip(self.offset);
        for (y, (idx, node)) in (area.y + 1..area.bottom()).zip(rows) {
            let icon = match (node.is_dir, node.expanded) {
                (true, true) => "▾ ",
                (true, false) => "▸ ",
                (false, _) => "  ",
            };
            let prefix = format!(" {}{icon}", "  ".repeat(node.depth));
            let name = truncate_middle(&node.name, width.saturating_sub(prefix.width()));
            let text = format!("{prefix}{name}");
            let mut style = if node.is_dir {
                directory_style
            } else {
                text_style
            };
            if is_focused && idx == self.selected {
                surface.set_style(Rect::new(area.x, y, area.width - 1, 1), selected_style);
                style = style.patch(selected_style);
            }
            surface.set_stringn(area.x, y, &text, width, style);
        }
    }

    /// Draws the selected row's path relative to the root, cut at the end with `…`.
    pub fn render_selected_path(&self, area: Rect, surface: &mut Surface, theme: &Theme) {
        let Some(node) = self.nodes.get(self.selected) else {
            return;
        };
        let path = node.path.strip_prefix(&self.root).unwrap_or(&node.path);
        let path = path.to_string_lossy();
        let width = usize::from(area.width);
        let style = theme.get("ui.text");
        surface.set_string_truncated(
            area.x,
            area.y,
            &path,
            width,
            |_| style,
            path.width() > width,
            false,
        );
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        count: usize,
        config: &FileExplorerConfig,
    ) -> TreeAction {
        let count = isize::try_from(count).unwrap_or(isize::MAX);
        let page = isize::try_from(self.height.max(1)).unwrap_or(isize::MAX);
        let half_page = (page / 2).max(1);
        match key {
            key!('j') | key!(Down) => self.move_by(count),
            key!('k') | key!(Up) => self.move_by(-count),
            ctrl!('d') => self.move_by(half_page),
            ctrl!('u') => self.move_by(-half_page),
            key!(PageDown) => self.move_by(page),
            key!(PageUp) => self.move_by(-page),
            key!(Home) => self.selected = 0,
            key!('G') | key!(End) => self.move_by(isize::MAX),
            key!('h') | key!(Left) => self.collapse_or_select_parent(),
            key!('l') | key!(Right) => match self.nodes.get(self.selected) {
                Some(node) if node.is_dir => self.expand_selected(config),
                Some(_) => return self.activate_selected(config),
                None => {}
            },
            key!(Enter) | key!('o') => {
                let action = self.activate_selected(config);
                if action != TreeAction::Consumed {
                    return action;
                }
            }
            key!('R') => self.refresh(config),
            key!(Esc) => return TreeAction::Unfocus,
            _ => return TreeAction::Ignored,
        }
        self.ensure_visible();
        TreeAction::Consumed
    }
}

/// Whether `command` may run while the sidebar has focus.
///
/// Only commands that don't edit the hidden buffer are allowed: typable commands,
/// macros, pickers and focus/buffer navigation. Everything else is ignored.
pub fn sidebar_allows(command: &MappableCommand) -> bool {
    let MappableCommand::Static { name, .. } = command else {
        return true;
    };
    name.ends_with("_picker")
        || name.starts_with("file_picker")
        || name.starts_with("file_explorer")
        || name.starts_with("jump_view_")
        || name.starts_with("rotate_view")
        || matches!(
            *name,
            "command_mode"
                | "command_palette"
                | "global_search"
                | "last_picker"
                | "goto_next_buffer"
                | "goto_previous_buffer"
                | "goto_last_accessed_file"
                | "goto_last_modified_file"
                | "toggle_sidebar"
                | "toggle_sidebar_focus"
                | "suspend"
        )
}

/// Fits `text` into `width` columns by replacing its middle with `…`.
fn truncate_middle(text: &str, width: usize) -> Cow<'_, str> {
    if text.width() <= width {
        return Cow::Borrowed(text);
    }
    let Some(width) = width.checked_sub(1) else {
        return Cow::Borrowed("");
    };
    let tail_width = width / 2;
    let head = &text[..fitting_len(text.chars(), width - tail_width)];
    let tail = &text[text.len() - fitting_len(text.chars().rev(), tail_width)..];
    Cow::Owned(format!("{head}…{tail}"))
}

/// Byte length of the leading `chars` that fit in `width` columns.
fn fitting_len(chars: impl Iterator<Item = char>, width: usize) -> usize {
    let mut used = 0;
    chars
        .take_while(|ch| {
            used += ch.width().unwrap_or(0);
            used <= width
        })
        .map(char::len_utf8)
        .sum()
}

/// Appends the entries of `dir` to `out`, recursing into dirs listed in `expanded`.
fn read_tree(
    dir: &Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    config: &FileExplorerConfig,
    out: &mut Vec<Node>,
) {
    let entries = directory_entries(dir, config).unwrap_or_default();
    for (path, is_dir) in entries {
        // Always hide VCS dirs, like the file pickers do.
        if matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some(".git" | ".pijul" | ".jj" | ".hg" | ".svn")
        ) {
            continue;
        }
        let name = path
            .strip_prefix(dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        let is_expanded = is_dir && expanded.contains(&path);
        out.push(Node {
            path: path.clone(),
            name,
            depth,
            is_dir,
            expanded: is_expanded,
        });
        if is_expanded {
            read_tree(&path, depth + 1, expanded, config, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir_all, File};

    use super::*;

    /// root/
    ///   .git/HEAD
    ///   a/b/deep.txt   (single-child chain, flattened to `a/b`)
    ///   dir/inner.txt
    ///   file.txt
    fn make_tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for dir in [".git", "a/b", "dir"] {
            create_dir_all(root.path().join(dir)).unwrap();
        }
        for file in [".git/HEAD", "a/b/deep.txt", "dir/inner.txt", "file.txt"] {
            File::create(root.path().join(file)).unwrap();
        }
        root
    }

    fn rows(tree: &FileTree) -> Vec<String> {
        tree.nodes
            .iter()
            .map(|node| format!("{}{}", "  ".repeat(node.depth), node.name))
            .collect()
    }

    fn new_tree(root: &tempfile::TempDir) -> (FileTree, FileExplorerConfig) {
        let config = FileExplorerConfig::default();
        (FileTree::new(root.path().to_path_buf(), &config), config)
    }

    #[test]
    fn dirs_first_flattened_and_vcs_hidden() {
        let root = make_tree();
        let (tree, _) = new_tree(&root);
        let flattened = format!("a{}b", std::path::MAIN_SEPARATOR);
        assert_eq!(rows(&tree), vec![flattened.as_str(), "dir", "file.txt"]);
    }

    #[test]
    fn expand_and_collapse() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.select(1);
        assert_eq!(tree.handle_key(key!('l'), 1, &config), TreeAction::Consumed);
        assert_eq!(rows(&tree)[1..], ["dir", "  inner.txt", "file.txt"]);

        // Enter on an expanded dir collapses it.
        assert_eq!(
            tree.handle_key(key!(Enter), 1, &config),
            TreeAction::Consumed
        );
        assert_eq!(rows(&tree)[1..], ["dir", "file.txt"]);
    }

    #[test]
    fn open_file_and_select_parent() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.select(1);
        tree.handle_key(key!('l'), 1, &config);
        tree.handle_key(key!('j'), 1, &config);
        assert_eq!(
            tree.handle_key(key!(Enter), 1, &config),
            TreeAction::Open(root.path().join("dir").join("inner.txt"))
        );

        // `h` on a file selects its parent; `h` again collapses it.
        tree.handle_key(key!('h'), 1, &config);
        assert_eq!(tree.selected, 1);
        tree.handle_key(key!('h'), 1, &config);
        assert_eq!(rows(&tree).len(), 3);
    }

    #[test]
    fn movement_is_clamped() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.handle_key(key!('k'), 1, &config);
        assert_eq!(tree.selected, 0);
        tree.handle_key(key!('j'), 10, &config);
        assert_eq!(tree.selected, 2);
        tree.handle_key(key!(Home), 1, &config);
        assert_eq!(tree.selected, 0);
        tree.handle_key(key!('G'), 1, &config);
        assert_eq!(tree.selected, 2);
        assert_eq!(tree.handle_key(key!(Esc), 1, &config), TreeAction::Unfocus);
        assert_eq!(tree.handle_key(key!('x'), 1, &config), TreeAction::Ignored);
    }

    #[test]
    fn refresh_keeps_expanded_dirs_and_selection() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.select(1);
        tree.handle_key(key!('l'), 1, &config);
        tree.select(3);
        File::create(root.path().join("dir/new.txt")).unwrap();
        File::create(root.path().join("0.txt")).unwrap();

        tree.handle_key(key!('R'), 1, &config);
        assert_eq!(
            rows(&tree)[1..],
            ["dir", "  inner.txt", "  new.txt", "0.txt", "file.txt"]
        );
        assert_eq!(tree.nodes[tree.selected].name, "file.txt");
    }

    #[test]
    fn render_rows() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.select(1);
        tree.handle_key(key!('l'), 1, &config);

        let area = Rect::new(0, 0, 16, 4);
        let mut surface = Surface::empty(area);
        tree.render(area, &mut surface, &Theme::default(), true);
        let lines: Vec<String> = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| surface[(x, y)].symbol.as_str())
                    .collect()
            })
            .collect();
        let title = root.path().file_name().unwrap().to_string_lossy();
        let title: String = format!(" {title}").chars().take(15).collect();
        assert_eq!(lines[0], format!("{title:<15}│"));
        assert_eq!(lines[1], format!("{:<15}│", " ▸ a/b"));
        assert_eq!(lines[2], format!("{:<15}│", " ▾ dir"));
        assert_eq!(lines[3], format!("{:<15}│", "     inner.txt"));
        // Three rows fit; the fourth row (file.txt) is scrolled out.
        assert_eq!(tree.height, 3);
    }

    #[test]
    fn allowlist_blocks_editing_commands() {
        for allowed in [
            MappableCommand::file_picker,
            MappableCommand::file_picker_in_current_directory,
            MappableCommand::buffer_picker,
            MappableCommand::command_mode,
            MappableCommand::goto_next_buffer,
            MappableCommand::jump_view_right,
            MappableCommand::toggle_sidebar_focus,
        ] {
            assert!(sidebar_allows(&allowed), "{}", allowed.name());
        }
        for blocked in [
            MappableCommand::delete_selection,
            MappableCommand::change_selection,
            MappableCommand::increment,
            MappableCommand::paste_after,
            MappableCommand::insert_mode,
            MappableCommand::undo,
        ] {
            assert!(!sidebar_allows(&blocked), "{}", blocked.name());
        }
        let typable: MappableCommand = ":buffer-goto 1".parse().unwrap();
        assert!(sidebar_allows(&typable));
    }

    #[test]
    fn scrolling_and_row_hits() {
        let root = make_tree();
        let (mut tree, config) = new_tree(&root);
        tree.set_height(2);
        tree.handle_key(key!('G'), 1, &config);
        assert_eq!(tree.offset, 1);
        assert_eq!(tree.row_at(0), Some(1));
        assert_eq!(tree.row_at(2), None);

        tree.scroll_by(-5);
        assert_eq!(tree.offset, 0);
        tree.scroll_by(5);
        assert_eq!(tree.offset, 1);
    }
}
