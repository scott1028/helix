use super::*;

use std::path::PathBuf;

use helix_stdx::path;
use helix_term::application::Application;
use helix_view::{
    document::Mode,
    editor::{BufferLine, SidebarConfig},
    tree::Direction,
};
use tempfile::NamedTempFile;
#[cfg(not(windows))]
use termina::event::{MouseButton, MouseEventKind};

fn current_path(app: &Application) -> PathBuf {
    helix_view::doc!(app.editor).path().unwrap().to_path_buf()
}

#[tokio::test(flavor = "multi_thread")]
async fn tab_switching_keys() -> anyhow::Result<()> {
    let files = [
        NamedTempFile::new()?,
        NamedTempFile::new()?,
        NamedTempFile::new()?,
    ];
    let paths: Vec<PathBuf> = files.iter().map(|f| path::normalize(f.path())).collect();
    let open_files = format!(
        ":o {}<ret>:o {}<ret>",
        files[1].path().to_string_lossy(),
        files[2].path().to_string_lossy()
    );

    let mut app = AppBuilder::new().with_file(files[0].path(), None).build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&open_files),
                Some(&|app| {
                    assert_eq!(3, app.editor.documents().count());
                    assert_eq!(paths[2], current_path(app));
                }),
            ),
            (
                Some("<A-2>"),
                Some(&|app| assert_eq!(paths[1], current_path(app))),
            ),
            // Index past the last tab clamps to the last one.
            (
                Some("<A-9>"),
                Some(&|app| assert_eq!(paths[2], current_path(app))),
            ),
            // Next from the last tab wraps around.
            (
                Some("<C-pagedown>"),
                Some(&|app| assert_eq!(paths[0], current_path(app))),
            ),
            (
                Some("<C-S-tab>"),
                Some(&|app| assert_eq!(paths[2], current_path(app))),
            ),
            (
                Some("<C-tab>"),
                Some(&|app| assert_eq!(paths[0], current_path(app))),
            ),
            (
                Some("<C-pageup>"),
                Some(&|app| assert_eq!(paths[2], current_path(app))),
            ),
            (
                Some(":buffer-goto 1<ret>"),
                Some(&|app| assert_eq!(paths[0], current_path(app))),
            ),
            (
                Some(":buffer-goto 0<ret>"),
                Some(&|app| {
                    assert!(app.editor.is_err());
                    assert_eq!(paths[0], current_path(app));
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn sidebar_toggle_shifts_views() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some("<space>t"),
                Some(&|app| {
                    assert!(app.editor.sidebar.visible);
                    assert_eq!(30, app.editor.tree.area().x);
                }),
            ),
            (
                Some("<space>t"),
                Some(&|app| {
                    assert!(!app.editor.sidebar.visible);
                    assert_eq!(0, app.editor.tree.area().x);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn sidebar_focus_blocks_edits() -> anyhow::Result<()> {
    let mut app = AppBuilder::new().with_input_text("#[h|]#ello\n").build()?;
    test_key_sequences(
        &mut app,
        vec![
            (
                Some("<C-e>"),
                Some(&|app| {
                    assert!(app.editor.sidebar.visible);
                    assert!(app.editor.sidebar.focused);
                }),
            ),
            (
                Some("d<A-d><C-a>c"),
                Some(&|app| {
                    assert!(app.editor.sidebar.focused);
                    assert_eq!(Mode::Normal, app.editor.mode);
                    assert_eq!("hello\n", helix_view::doc!(app.editor).text().to_string());
                }),
            ),
            (
                Some("<C-e>"),
                Some(&|app| assert!(!app.editor.sidebar.focused)),
            ),
            (
                Some("<C-e><esc>"),
                Some(&|app| assert!(!app.editor.sidebar.focused)),
            ),
            (
                Some("<C-e><C-w>l"),
                Some(&|app| assert!(!app.editor.sidebar.focused)),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn sidebar_spatial_navigation() -> anyhow::Result<()> {
    fn is_leftmost(app: &Application) -> bool {
        let tree = &app.editor.tree;
        tree.find_split_in_direction(tree.focus, Direction::Left)
            .is_none()
    }

    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            // Hidden sidebar: nothing to the left.
            (
                Some("<C-w>h"),
                Some(&|app| assert!(!app.editor.sidebar.focused)),
            ),
            // From the right split, C-w h goes to the left split first.
            (
                Some("<space>t:vsplit<ret><C-w>h"),
                Some(&|app| {
                    assert!(!app.editor.sidebar.focused);
                    assert!(is_leftmost(app));
                }),
            ),
            (
                Some("<C-w>h"),
                Some(&|app| assert!(app.editor.sidebar.focused)),
            ),
            (
                Some("<C-w>l"),
                Some(&|app| {
                    assert!(!app.editor.sidebar.focused);
                    assert!(is_leftmost(app));
                }),
            ),
            // Close the extra split so the harness can quit.
            (
                Some("<C-w>o"),
                Some(&|app| assert_eq!(1, app.editor.tree.views().count())),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn sidebar_leaves_select_mode_and_unfocuses_on_tab_switch() -> anyhow::Result<()> {
    let file = NamedTempFile::new()?;
    let path = path::normalize(file.path());
    let mut app = AppBuilder::new().with_file(file.path(), None).build()?;
    test_key_sequences(
        &mut app,
        vec![
            (
                Some("v<C-e>"),
                Some(&|app| {
                    assert!(app.editor.sidebar.focused);
                    assert_eq!(Mode::Normal, app.editor.mode);
                }),
            ),
            (
                Some(":new<ret>"),
                Some(&|app| {
                    assert!(!app.editor.sidebar.focused);
                    assert_eq!(2, app.editor.documents().count());
                }),
            ),
            (
                Some("<C-e><A-1>"),
                Some(&|app| {
                    assert!(!app.editor.sidebar.focused);
                    assert_eq!(path, current_path(app));
                }),
            ),
        ],
        false,
    )
    .await
}

/// Sends keys (helix key syntax) and mouse events in order, then waits until idle.
#[cfg(not(windows))]
async fn send_input(app: &mut Application, keys: &str, clicks: &[(MouseEventKind, u16, u16)]) {
    use termina::event::{Event, KeyEvent, Modifiers, MouseEvent};

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    for key in helix_view::input::parse_macro(keys).unwrap() {
        tx.send(Ok(Event::Key(KeyEvent::from(key)))).unwrap();
    }
    for &(kind, column, row) in clicks {
        let mouse = MouseEvent {
            kind,
            column,
            row,
            modifiers: Modifiers::NONE,
        };
        tx.send(Ok(Event::Mouse(mouse))).unwrap();
    }
    app.event_loop_until_idle(&mut rx_stream).await;
}

#[cfg(not(windows))]
fn click(button: MouseButton, column: u16, row: u16) -> [(MouseEventKind, u16, u16); 2] {
    [
        (MouseEventKind::Down(button), column, row),
        (MouseEventKind::Up(button), column, row),
    ]
}

#[cfg(not(windows))]
#[tokio::test(flavor = "multi_thread")]
async fn tab_mouse_clicks() -> anyhow::Result<()> {
    let files = [NamedTempFile::new()?, NamedTempFile::new()?];
    let paths: Vec<PathBuf> = files.iter().map(|f| path::normalize(f.path())).collect();
    // Tab text is " {file name} ".
    let first_tab_width = paths[0].file_name().unwrap().len() as u16 + 2;

    let config = Config {
        editor: helix_view::editor::Config {
            bufferline: BufferLine::Always,
            ..test_editor_config()
        },
        ..test_config()
    };
    let mut app = AppBuilder::new()
        .with_config(config)
        .with_file(files[0].path(), None)
        .build()?;

    let open_second = format!(":o {}<ret>", files[1].path().to_string_lossy());
    send_input(&mut app, &open_second, &[]).await;
    assert_eq!(paths[1], current_path(&app));

    // Left click on the first tab switches to it.
    send_input(&mut app, "", &click(MouseButton::Left, 1, 0)).await;
    assert_eq!(paths[0], current_path(&app));

    // Middle click on the second tab closes it.
    send_input(
        &mut app,
        "",
        &click(MouseButton::Middle, first_tab_width + 1, 0),
    )
    .await;
    assert_eq!(1, app.editor.documents().count());

    // Middle click on a modified tab keeps it and shows an error.
    send_input(&mut app, "ihello<esc>", &click(MouseButton::Middle, 1, 0)).await;
    assert_eq!(1, app.editor.documents().count());
    assert!(app.editor.is_err());

    // Send a key so the harness sees the app go idle before quitting.
    test_key_sequence(&mut app, Some("<esc>"), None, false).await
}

#[cfg(not(windows))]
#[tokio::test(flavor = "multi_thread")]
async fn sidebar_mouse_focus() -> anyhow::Result<()> {
    let config = Config {
        editor: helix_view::editor::Config {
            sidebar: SidebarConfig {
                visible: true,
                ..Default::default()
            },
            ..test_editor_config()
        },
        ..test_config()
    };
    let mut app = AppBuilder::new().with_config(config).build()?;

    // Render once so the sidebar area is known.
    send_input(&mut app, "<esc>", &[]).await;
    assert_eq!(30, app.editor.tree.area().x);

    // Clicking the sidebar title focuses the sidebar; scrolling keeps it.
    let mut events = click(MouseButton::Left, 2, 0).to_vec();
    events.push((MouseEventKind::ScrollDown, 2, 3));
    send_input(&mut app, "", &events).await;
    assert!(app.editor.sidebar.focused);

    // Clicking the editor view takes focus back.
    send_input(&mut app, "", &click(MouseButton::Left, 60, 5)).await;
    assert!(!app.editor.sidebar.focused);

    // Send a key so the harness sees the app go idle before quitting.
    test_key_sequence(&mut app, Some("<esc>"), None, false).await
}
