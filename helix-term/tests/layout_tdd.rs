//! TDD integration tests for the sidebar width: widen on focus and border drag.
//! Run with `cargo test --features integration --profile integration --test layout_tdd`.

#[cfg(feature = "integration")]
mod test {
    #[allow(dead_code)]
    mod helpers;

    use helix_term::{application::Application, config::Config};
    use helix_view::editor::SidebarConfig;

    use self::helpers::*;

    fn sidebar_config(width: u16) -> Config {
        Config {
            editor: helix_view::editor::Config {
                sidebar: SidebarConfig {
                    visible: true,
                    width,
                },
                ..test_editor_config()
            },
            ..test_config()
        }
    }

    /// The views start right after the sidebar.
    fn sidebar_width(app: &Application) -> u16 {
        app.editor.tree.area().x
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn focus_widens_sidebar_to_fit_names() -> anyhow::Result<()> {
        // The tree root is helix-term/, whose rows need more than 10 columns.
        let mut app = AppBuilder::new().with_config(sidebar_config(10)).build()?;
        test_key_sequences(
            &mut app,
            vec![
                (
                    Some("<esc>"),
                    Some(&|app| assert_eq!(10, sidebar_width(app))),
                ),
                (
                    Some("<C-e>"),
                    Some(&|app| {
                        let width = sidebar_width(app);
                        // Test screens are 120 columns; the cap is half of that.
                        assert!(width > 10 && width <= 60, "{width}");
                    }),
                ),
                (
                    Some("<C-e>"),
                    Some(&|app| assert_eq!(10, sidebar_width(app))),
                ),
            ],
            false,
        )
        .await
    }

    /// Sends keys (helix key syntax), then mouse events, and waits until idle.
    #[cfg(not(windows))]
    async fn send_input(
        app: &mut Application,
        keys: &str,
        mouse: &[(termina::event::MouseEventKind, u16, u16)],
    ) {
        use termina::event::{Event, KeyEvent, Modifiers, MouseEvent};

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut rx_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        for key in helix_view::input::parse_macro(keys).unwrap() {
            tx.send(Ok(Event::Key(KeyEvent::from(key)))).unwrap();
        }
        for &(kind, column, row) in mouse {
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

    /// Left-button drag along row 5 from column `from` to column `to`.
    #[cfg(not(windows))]
    fn drag(from: u16, to: u16) -> [(termina::event::MouseEventKind, u16, u16); 3] {
        use termina::event::{MouseButton, MouseEventKind};
        [
            (MouseEventKind::Down(MouseButton::Left), from, 5),
            (MouseEventKind::Drag(MouseButton::Left), to, 5),
            (MouseEventKind::Up(MouseButton::Left), to, 5),
        ]
    }

    #[cfg(not(windows))]
    #[tokio::test(flavor = "multi_thread")]
    async fn dragging_border_sets_restore_width() -> anyhow::Result<()> {
        let mut app = AppBuilder::new().with_config(sidebar_config(30)).build()?;
        send_input(&mut app, "<esc>", &[]).await;
        assert_eq!(30, sidebar_width(&app));

        // The border is the last sidebar column; dragging it keeps focus in the editor.
        send_input(&mut app, "", &drag(29, 40)).await;
        assert_eq!(41, sidebar_width(&app));
        assert!(!app.editor.sidebar.focused);

        // Capped at half of the 120-column test screen.
        send_input(&mut app, "", &drag(40, 110)).await;
        assert_eq!(60, sidebar_width(&app));

        // Focusing and leaving the sidebar goes back to the dragged width.
        send_input(&mut app, "", &drag(59, 20)).await;
        send_input(&mut app, "<C-e><C-e>", &[]).await;
        assert_eq!(21, sidebar_width(&app));

        // Send a key so the harness sees the app go idle before quitting.
        test_key_sequence(&mut app, Some("<esc>"), None, false).await
    }

    #[cfg(not(windows))]
    #[tokio::test(flavor = "multi_thread")]
    async fn drag_while_focused_holds_until_unfocused() -> anyhow::Result<()> {
        let mut app = AppBuilder::new().with_config(sidebar_config(10)).build()?;
        send_input(&mut app, "<C-e>", &[]).await;
        let wide = sidebar_width(&app);
        assert!(wide > 12, "{wide}");

        // A drag narrower than the names wins over the auto width while focused.
        send_input(&mut app, "", &drag(wide - 1, 11)).await;
        assert!(app.editor.sidebar.focused);
        assert_eq!(12, sidebar_width(&app));

        send_input(&mut app, "<C-e>", &[]).await;
        assert_eq!(12, sidebar_width(&app));
        // The next focus widens again.
        send_input(&mut app, "<C-e>", &[]).await;
        assert_eq!(wide, sidebar_width(&app));

        // Send a key so the harness sees the app go idle before quitting.
        test_key_sequence(&mut app, Some("<esc>"), None, false).await
    }
}
