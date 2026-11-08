//! TDD integration tests for `space v`: open the Markdown file in an external previewer.
//! Run with `cargo test --features integration --profile integration --test markdown_preview_tdd`.

#[cfg(all(feature = "integration", unix))]
mod test {
    #[allow(dead_code)]
    mod helpers;

    use std::path::Path;

    use helix_term::{application::Application, config::Config};
    use helix_view::editor::Severity;

    use self::helpers::*;

    fn preview_app(previewer: &[&str], file: &Path) -> anyhow::Result<Application> {
        let previewer = previewer.iter().map(|arg| arg.to_string()).collect();
        preview_app_with(previewer, file)
    }

    fn preview_app_with(previewer: Vec<String>, file: &Path) -> anyhow::Result<Application> {
        let config = Config {
            editor: helix_view::editor::Config {
                markdown_preview: previewer,
                ..test_editor_config()
            },
            ..test_config()
        };
        AppBuilder::new()
            .with_config(config)
            .with_file(file, None)
            .build()
    }

    fn markdown_file(content: &str) -> anyhow::Result<tempfile::NamedTempFile> {
        let mut file = tempfile::Builder::new().suffix(".md").tempfile()?;
        std::io::Write::write_all(&mut file, content.as_bytes())?;
        Ok(file)
    }

    /// A previewer that writes the path it gets into `out`.
    fn recording_previewer(out: &Path) -> Vec<String> {
        let script = r#"echo "$1" > "$2""#;
        let out = out.to_str().unwrap();
        ["sh", "-c", script, "sh", "%{file_path_absolute}", out]
            .map(String::from)
            .to_vec()
    }

    /// The previewer runs detached, so wait for its output.
    async fn read_recorded_path(out: &Path) -> Option<String> {
        for _ in 0..50 {
            if let Ok(path) = std::fs::read_to_string(out) {
                if !path.is_empty() {
                    return Some(path.trim_end().to_string());
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        None
    }

    fn infobox_title(app: &Application) -> Option<&str> {
        app.editor.autoinfo.as_ref().map(|info| info.title.as_ref())
    }

    fn is_modified(app: &Application) -> bool {
        helix_view::current_ref!(app.editor).1.is_modified()
    }

    fn error_message(app: &Application) -> String {
        match app.editor.get_status() {
            Some((message, Severity::Error)) => message.to_string(),
            status => panic!("expected an error, got {status:?}"),
        }
    }

    /// Text of the infobox rows drawn as inactive.
    fn inactive_rows(app: &Application) -> Vec<String> {
        let info = app.editor.autoinfo.as_ref().expect("infobox is shown");
        let lines: Vec<&str> = info.text.lines().collect();
        info.inactive_rows
            .iter()
            .map(|&row| lines[row].trim().to_string())
            .collect()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_previewer_greys_out_space_v() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let mut app = preview_app(&["hx-no-such-previewer"], file.path())?;
        test_key_sequence(
            &mut app,
            Some("<space>"),
            Some(&|app| {
                let rows = inactive_rows(app);
                assert_eq!(1, rows.len(), "{rows:?}");
                assert!(rows[0].starts_with('v'), "{rows:?}");
                assert!(rows[0].ends_with("Preview Markdown file in external program"));
            }),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn found_previewer_keeps_space_v_active() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let mut app = preview_app(&["sh"], file.path())?;
        test_key_sequence(
            &mut app,
            Some("<space>"),
            Some(&|app| assert_eq!(Vec::<String>::new(), inactive_rows(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn non_markdown_file_shows_error() -> anyhow::Result<()> {
        let file = tempfile::Builder::new().suffix(".rs").tempfile()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;
        test_key_sequence(
            &mut app,
            Some("<space>v"),
            Some(&|app| assert_eq!("Not a Markdown file", error_message(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_previewer_shows_error() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let mut app = preview_app(
            &["hx-no-such-previewer", "%{file_path_absolute}"],
            file.path(),
        )?;
        test_key_sequence(
            &mut app,
            Some("<space>v"),
            Some(&|app| {
                assert_eq!(
                    "`hx-no-such-previewer` not found in PATH",
                    error_message(app)
                )
            }),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scratch_buffer_shows_error() -> anyhow::Result<()> {
        let config = Config {
            editor: helix_view::editor::Config {
                markdown_preview: vec!["sh".to_string()],
                ..test_editor_config()
            },
            ..test_config()
        };
        let mut app = AppBuilder::new().with_config(config).build()?;
        test_key_sequence(
            &mut app,
            Some(":set-language markdown<ret><space>v"),
            Some(&|app| assert_eq!("Save the buffer to a file first", error_message(app))),
            false,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn saved_file_opens_previewer_with_absolute_path() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let out_dir = tempfile::tempdir()?;
        let out = out_dir.path().join("previewed");
        let mut app = preview_app_with(recording_previewer(&out), file.path())?;
        test_key_sequence(&mut app, Some("<space>v"), None, false).await?;

        let expected = helix_stdx::path::canonicalize(file.path());
        assert_eq!(
            Some(expected.to_string_lossy().into_owned()),
            read_recorded_path(&out).await
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unsaved_changes_ask_first_and_n_cancels() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let out_dir = tempfile::tempdir()?;
        let out = out_dir.path().join("previewed");
        let mut app = preview_app_with(recording_previewer(&out), file.path())?;
        test_key_sequences(
            &mut app,
            vec![
                (
                    Some("ione<esc><space>v"),
                    Some(&|app| {
                        assert_eq!(Some("Save changes to preview?"), infobox_title(app));
                        assert!(is_modified(app));
                    }),
                ),
                (
                    Some("n"),
                    Some(&|app| {
                        assert_eq!(None, infobox_title(app));
                        assert!(is_modified(app));
                    }),
                ),
            ],
            false,
        )
        .await?;

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(!out.exists(), "the previewer must not run");
        assert_eq!("# Title\n", std::fs::read_to_string(file.path())?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unsaved_changes_y_saves_then_previews() -> anyhow::Result<()> {
        let file = markdown_file("# Title\n")?;
        let out_dir = tempfile::tempdir()?;
        let out = out_dir.path().join("previewed");
        let mut app = preview_app_with(recording_previewer(&out), file.path())?;
        test_key_sequences(
            &mut app,
            vec![
                (Some("ione<esc><space>v"), None),
                (
                    Some("y"),
                    Some(&|app| {
                        assert_eq!(None, infobox_title(app));
                        assert!(!is_modified(app));
                    }),
                ),
            ],
            false,
        )
        .await?;

        assert_eq!("one# Title\n", std::fs::read_to_string(file.path())?);
        let expected = helix_stdx::path::canonicalize(file.path());
        assert_eq!(
            Some(expected.to_string_lossy().into_owned()),
            read_recorded_path(&out).await
        );
        Ok(())
    }
}
