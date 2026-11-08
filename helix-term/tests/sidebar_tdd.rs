//! TDD tests for sidebar name truncation, auto width and the selected path row.

use std::{
    fs::{create_dir_all, File},
    path::MAIN_SEPARATOR,
};

use helix_term::{key, ui::sidebar::FileTree};
use helix_view::{editor::FileExplorerConfig, graphics::Rect, Theme};
use tui::buffer::Buffer as Surface;

const LONG_NAME: &str = "preview-spec_01-claude-opus-20261026123341.txt";

/// Creates `paths` under a temp dir (a trailing `/` makes a dir) and loads the tree.
fn new_tree(paths: &[&str]) -> (tempfile::TempDir, FileTree) {
    let root = tempfile::tempdir().unwrap();
    for path in paths {
        match path.strip_suffix('/') {
            Some(dir) => create_dir_all(root.path().join(dir)).unwrap(),
            None => drop(File::create(root.path().join(path)).unwrap()),
        }
    }
    let tree = FileTree::new(root.path().to_path_buf(), &FileExplorerConfig::default());
    (root, tree)
}

fn lines(surface: &Surface) -> Vec<String> {
    let area = surface.area;
    (area.top()..area.bottom())
        .map(|y| {
            (area.left()..area.right())
                .map(|x| surface[(x, y)].symbol.as_str())
                .collect()
        })
        .collect()
}

fn render_rows(tree: &mut FileTree, width: u16, height: u16) -> Vec<String> {
    let area = Rect::new(0, 0, width, height);
    let mut surface = Surface::empty(area);
    tree.render(area, &mut surface, &Theme::default(), false);
    lines(&surface)
}

#[test]
fn long_names_are_cut_in_the_middle() {
    let (_root, mut tree) = new_tree(&[LONG_NAME, "short.txt"]);
    let rows = render_rows(&mut tree, 20, 3);
    // 19 text columns: " " + file icon "  " + 16 for the name.
    assert_eq!(rows[1], "   preview-…341.txt│");
    assert_eq!(rows[2], "   short.txt       │");
}

#[test]
fn wide_chars_are_cut_within_the_width() {
    let (_root, mut tree) = new_tree(&["檔案名稱很長的檔案.txt"]);
    let rows = render_rows(&mut tree, 14, 2);
    // 10 columns for the name; a wide char fills its cell and blanks the next one.
    assert_eq!(rows[1], "   檔 案 ….txt │");
}

#[test]
fn content_width_fits_the_widest_row() {
    let long_child = format!("dir/{LONG_NAME}");
    let (_root, mut tree) = new_tree(&["dir/", &long_child, "a.txt"]);
    let collapsed = tree.content_width();

    tree.select(0);
    tree.handle_key(key!('l'), 1, &FileExplorerConfig::default());
    // " " + indent "  " + file icon "  " + name, then one padding column and the border.
    assert_eq!(usize::from(tree.content_width()), LONG_NAME.len() + 7);
    assert!(collapsed < tree.content_width());
}

fn render_selected_path(tree: &FileTree, width: u16) -> String {
    let area = Rect::new(0, 0, width, 1);
    let mut surface = Surface::empty(area);
    tree.render_selected_path(area, &mut surface, &Theme::default());
    lines(&surface).remove(0)
}

#[test]
fn selected_path_is_relative_and_cut_at_the_end() {
    let long_child = format!("dir/{LONG_NAME}");
    let (_root, mut tree) = new_tree(&["dir/", &long_child]);
    tree.select(0);
    tree.handle_key(key!('l'), 1, &FileExplorerConfig::default());
    tree.handle_key(key!('j'), 1, &FileExplorerConfig::default());
    let path = format!("dir{MAIN_SEPARATOR}{LONG_NAME}");

    assert_eq!(render_selected_path(&tree, 60), format!("{path:<60}"));
    let exact_width = u16::try_from(path.len()).unwrap();
    assert_eq!(render_selected_path(&tree, exact_width), path);
    let head: String = path.chars().take(19).collect();
    assert_eq!(render_selected_path(&tree, 20), format!("{head}…"));
}
