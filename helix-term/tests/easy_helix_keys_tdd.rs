//! TDD integration tests for the easy-helix default keys (`keymap::easy_helix()`).
//! Run with `cargo test --features integration --profile integration --test easy_helix_keys_tdd`.

#[cfg(feature = "integration")]
mod test {
    #[allow(dead_code)]
    mod helpers;

    use helix_term::{application::Application, config::Config};

    use self::helpers::*;

    fn easy_helix_config() -> Config {
        Config {
            keys: helix_term::keymap::easy_helix(),
            ..test_config()
        }
    }

    fn app_with_text(text: &str) -> anyhow::Result<Application> {
        AppBuilder::new()
            .with_config(easy_helix_config())
            .with_input_text(text)
            .build()
    }

    fn text(app: &Application) -> String {
        helix_view::doc!(app.editor).text().to_string()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dd_deletes_the_line() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\nbar\n")?;
        test_key_sequence(
            &mut app,
            Some("dd"),
            Some(&|app| assert_eq!("bar\n", text(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn yy_then_p_duplicates_the_line() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\nbar\n")?;
        test_key_sequence(
            &mut app,
            Some("yyp"),
            Some(&|app| assert_eq!("foo\nfoo\nbar\n", text(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_r_redoes() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequence(
            &mut app,
            Some("ione<esc>u<C-r>"),
            Some(&|app| assert_eq!("onefoo\n", text(app))),
            false,
        )
        .await
    }

    /// The harness quits with a single `:q!`, so close the extra split first.
    #[allow(clippy::type_complexity)]
    const CLOSE_SPLIT: (Option<&str>, Option<&dyn Fn(&Application)>) = (Some(":q!<ret>"), None);

    fn view_count(app: &Application) -> usize {
        app.editor.tree.views().count()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_w_closes_the_tab_without_exiting() -> anyhow::Result<()> {
        let first = temp_file_with_contents("first\n")?;
        let second = temp_file_with_contents("second\n")?;
        let mut app = AppBuilder::new()
            .with_config(easy_helix_config())
            .with_file(first.path(), None)
            .with_file(second.path(), None)
            .build()?;
        test_key_sequence(
            &mut app,
            Some("<C-w>"),
            Some(&|app| assert_eq!(1, app.editor.documents().count())),
            false,
        )
        .await
    }

    fn app_with_two_files(
        first: &tempfile::NamedTempFile,
        second: &tempfile::NamedTempFile,
    ) -> anyhow::Result<Application> {
        AppBuilder::new()
            .with_config(easy_helix_config())
            .with_file(first.path(), None)
            .with_file(second.path(), None)
            .build()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alt_right_and_alt_left_switch_tabs() -> anyhow::Result<()> {
        let first = temp_file_with_contents("first\n")?;
        let second = temp_file_with_contents("second\n")?;
        let mut app = app_with_two_files(&first, &second)?;
        let initial = text(&app);
        test_key_sequences(
            &mut app,
            vec![
                (
                    Some("<A-right>"),
                    Some(&|app| assert_ne!(initial, text(app))),
                ),
                (
                    Some("<A-left>"),
                    Some(&|app| assert_eq!(initial, text(app))),
                ),
            ],
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn select_mode_alt_left_does_not_switch_tabs() -> anyhow::Result<()> {
        let first = temp_file_with_contents("first\n")?;
        let second = temp_file_with_contents("second\n")?;
        let mut app = app_with_two_files(&first, &second)?;
        let initial = text(&app);
        test_key_sequence(
            &mut app,
            Some("v<A-left>"),
            Some(&|app| assert_eq!(initial, text(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn f5_reloads_the_file_from_disk() -> anyhow::Result<()> {
        let file = temp_file_with_contents("one\n")?;
        let mut app = AppBuilder::new()
            .with_config(easy_helix_config())
            .with_file(file.path(), None)
            .build()?;
        std::fs::write(file.path(), "two\n")?;
        test_key_sequence(
            &mut app,
            Some("<F5>"),
            Some(&|app| assert_eq!("two\n", text(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_n_opens_a_new_split() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequences(
            &mut app,
            vec![
                (Some("<C-n>"), Some(&|app| assert_eq!(2, view_count(app)))),
                CLOSE_SPLIT,
            ],
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_backslash_splits_vertically() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequences(
            &mut app,
            vec![
                (Some("<C-\\>"), Some(&|app| assert_eq!(2, view_count(app)))),
                CLOSE_SPLIT,
            ],
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_left_focuses_the_left_split() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequences(
            &mut app,
            vec![
                (
                    Some(":vsplit<ret><C-left>"),
                    Some(&|app| {
                        let leftmost = app.editor.tree.views().map(|(view, _)| view.area.x).min();
                        let focused = app.editor.tree.get(app.editor.tree.focus).area.x;
                        assert_eq!(leftmost, Some(focused));
                    }),
                ),
                CLOSE_SPLIT,
            ],
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn select_mode_d_still_deletes_the_selection() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequence(
            &mut app,
            Some("vld"),
            Some(&|app| assert_eq!("o\n", text(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_q_quits_even_with_unsaved_changes() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequence(&mut app, Some("ione<esc><C-q>"), None, true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alt_w_closes_the_tab_dropping_changes() -> anyhow::Result<()> {
        let first = temp_file_with_contents("first\n")?;
        let second = temp_file_with_contents("second\n")?;
        let mut app = AppBuilder::new()
            .with_config(easy_helix_config())
            .with_file(first.path(), None)
            .with_file(second.path(), None)
            .build()?;
        test_key_sequence(
            &mut app,
            Some("ione<esc><A-w>"),
            Some(&|app| {
                assert_eq!(1, app.editor.documents().count());
                assert!(!helix_view::doc!(app.editor).is_modified());
            }),
            false,
        )
        .await?;
        assert_eq!("first\n", std::fs::read_to_string(first.path())?);
        assert_eq!("second\n", std::fs::read_to_string(second.path())?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ctrl_x_writes_and_quits() -> anyhow::Result<()> {
        let file = temp_file_with_contents("foo\n")?;
        let mut app = AppBuilder::new()
            .with_config(easy_helix_config())
            .with_file(file.path(), None)
            .build()?;
        test_key_sequence(&mut app, Some("ione<esc><C-x>"), None, true).await?;
        assert_eq!("onefoo\n", std::fs::read_to_string(file.path())?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shift_l_toggles_line_numbers() -> anyhow::Result<()> {
        use helix_view::editor::GutterType;

        let mut app = app_with_text("#[f|]#oo\n")?;
        let has_line_numbers = |app: &Application| {
            app.editor
                .config()
                .gutters
                .layout
                .contains(&GutterType::LineNumbers)
        };
        test_key_sequences(
            &mut app,
            vec![
                (Some("L"), Some(&|app| assert!(!has_line_numbers(app)))),
                (Some("L"), Some(&|app| assert!(has_line_numbers(app)))),
            ],
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shift_w_toggles_soft_wrap() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequence(
            &mut app,
            Some("W"),
            Some(&|app| assert_eq!(Some(true), app.editor.config().soft_wrap.enable)),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alt_l_then_m_sets_markdown() -> anyhow::Result<()> {
        let mut app = app_with_text("#[f|]#oo\n")?;
        test_key_sequence(
            &mut app,
            Some("<A-l>m"),
            Some(&|app| {
                assert_eq!(
                    Some("markdown"),
                    helix_view::doc!(app.editor).language_name()
                )
            }),
            false,
        )
        .await
    }

    #[test]
    fn bufferline_shows_with_multiple_buffers_by_default() {
        use helix_view::editor::BufferLine;

        assert_eq!(
            BufferLine::Multiple,
            helix_view::editor::Config::default().bufferline
        );
    }
}
