mod common;

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;
use subx_cli::cli::TranslateArgs;
use subx_cli::commands::translate_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct DynamicTranslationResponder;

impl Respond for DynamicTranslationResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value =
            serde_json::from_slice(&request.body).expect("request body should be JSON");
        let prompt = body["messages"]
            .as_array()
            .and_then(|messages| messages.last())
            .and_then(|message| message["content"].as_str())
            .unwrap_or("");

        let content = if prompt.contains("Cues to translate:") {
            let translations: Vec<serde_json::Value> = prompt
                .lines()
                .filter_map(|line| line.trim().strip_prefix("- id: "))
                .enumerate()
                .map(|(idx, id)| {
                    json!({
                        "id": id.trim(),
                        "text": format!("翻譯後第{}句", idx + 1),
                    })
                })
                .collect();
            json!({ "translations": translations }).to_string()
        } else {
            json!({
                "terms": [
                    { "source": "Alice", "target": "愛麗絲" }
                ]
            })
            .to_string()
        };

        ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                {
                    "message": { "content": content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
        }))
    }
}

fn base_args(input: PathBuf) -> TranslateArgs {
    TranslateArgs {
        paths: vec![input],
        input_paths: vec![],
        recursive: false,
        target_language: Some("zh-TW".to_string()),
        source_language: Some("en".to_string()),
        glossary: None,
        context: Some("Use natural subtitles".to_string()),
        output: None,
        no_extract: false,
        force: false,
        replace: false,
    }
}

async fn mock_config() -> (MockServer, subx_cli::config::TestConfigService) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer mock-api-key"))
        .and(header("content-type", "application/json"))
        .respond_with(DynamicTranslationResponder)
        .mount(&server)
        .await;
    let config = TestConfigBuilder::new()
        .with_mock_ai_server(&server.uri())
        .with_translation_batch_size(10)
        .build_service();
    (server, config)
}

async fn mock_config_with_backup() -> (MockServer, subx_cli::config::TestConfigService) {
    let (server, _config) = mock_config().await;
    let config = TestConfigBuilder::new()
        .with_mock_ai_server(&server.uri())
        .with_translation_batch_size(10)
        .with_backup_enabled(true)
        .build_service();
    (server, config)
}

fn create_zip(zip_path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(zip_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
}

#[tokio::test]
async fn translate_command_writes_default_suffixed_output() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("movie.srt");
    std::fs::write(
        &input,
        "1\n00:00:01,000 --> 00:00:02,000\nHello Alice\n\n2\n00:00:03,000 --> 00:00:04,000\nBye Alice\n",
    )
    .unwrap();
    let output = tmp.path().join("movie.zh-TW.srt");
    let (_server, config) = mock_config().await;

    translate_command::execute(base_args(input.clone()), &config)
        .await
        .expect("translation command should succeed");

    let translated = std::fs::read_to_string(output).expect("translated output");
    assert!(translated.contains("翻譯後第1句"));
    assert!(translated.contains("翻譯後第2句"));
    let original = std::fs::read_to_string(input).expect("original remains");
    assert!(original.contains("Hello Alice"));
}

#[tokio::test]
async fn translate_command_rejects_existing_output_without_force() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("movie.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let output = tmp.path().join("movie.zh-TW.srt");
    std::fs::write(&output, "existing").unwrap();
    let (_server, config) = mock_config().await;

    let err = translate_command::execute(base_args(input), &config)
        .await
        .expect_err("existing output should fail without --force");

    assert!(err.to_string().contains("File already exists"));
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing");
}

#[tokio::test]
async fn translate_command_force_overwrites_existing_output() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("movie.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let output = tmp.path().join("movie.zh-TW.srt");
    std::fs::write(&output, "existing").unwrap();
    let (_server, config) = mock_config().await;
    let mut args = base_args(input);
    args.force = true;

    translate_command::execute(args, &config)
        .await
        .expect("--force should overwrite existing output");

    let translated = std::fs::read_to_string(output).unwrap();
    assert!(translated.contains("翻譯後第1句"));
    assert!(!translated.contains("existing"));
}

#[tokio::test]
async fn translate_command_writes_batch_outputs_to_directory() {
    let tmp = tempdir().expect("tempdir");
    let input_dir = tmp.path().join("subs");
    let output_dir = tmp.path().join("translated");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(
        input_dir.join("a.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nHello A\n",
    )
    .unwrap();
    std::fs::write(
        input_dir.join("b.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nHello B\n",
    )
    .unwrap();
    let (_server, config) = mock_config().await;
    let mut args = base_args(input_dir);
    args.output = Some(output_dir.clone());

    translate_command::execute(args, &config)
        .await
        .expect("batch directory translation should succeed");

    assert!(
        std::fs::read_to_string(output_dir.join("a.zh-TW.srt"))
            .unwrap()
            .contains("翻譯後第1句")
    );
    assert!(
        std::fs::read_to_string(output_dir.join("b.zh-TW.srt"))
            .unwrap()
            .contains("翻譯後第1句")
    );
}

#[tokio::test]
async fn translate_command_replace_mode_creates_backup() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("movie.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let (_server, config) = mock_config_with_backup().await;
    let mut args = base_args(input.clone());
    args.replace = true;

    translate_command::execute(args, &config)
        .await
        .expect("replace mode should succeed");

    let translated_source = std::fs::read_to_string(&input).unwrap();
    assert!(translated_source.contains("翻譯後第1句"));
    let backup = tmp.path().join("movie.srt.backup");
    assert!(std::fs::read_to_string(backup).unwrap().contains("Hello"));
}

#[tokio::test]
async fn translate_command_continues_after_per_file_failure() {
    let tmp = tempdir().expect("tempdir");
    let input_dir = tmp.path().join("subs");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(
        input_dir.join("good.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
    )
    .unwrap();
    std::fs::write(input_dir.join("bad.srt"), "this is not valid srt").unwrap();
    let (_server, config) = mock_config().await;

    let err = translate_command::execute(base_args(input_dir), &config)
        .await
        .expect_err("invalid file should be reported after processing other files");

    assert!(err.to_string().contains("translation job(s) failed"));
    assert!(
        std::fs::read_to_string(tmp.path().join("subs/good.zh-TW.srt"))
            .unwrap()
            .contains("翻譯後第1句")
    );
}

#[tokio::test]
async fn translate_command_writes_archive_origin_output_beside_archive() {
    let tmp = tempdir().expect("tempdir");
    let archive = tmp.path().join("subs.zip");
    create_zip(
        &archive,
        &[(
            "inside.srt",
            b"1\n00:00:01,000 --> 00:00:02,000\nHello from archive\n",
        )],
    );
    let (_server, config) = mock_config().await;

    translate_command::execute(base_args(archive), &config)
        .await
        .expect("archive input should translate extracted subtitle");

    let output = tmp.path().join("inside.zh-TW.srt");
    assert!(
        std::fs::read_to_string(output)
            .unwrap()
            .contains("翻譯後第1句")
    );
}
