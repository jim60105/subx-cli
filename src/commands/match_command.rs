use crate::Result;
use crate::cli::MatchArgs;
use crate::cli::display_match_results;
use crate::config::init_config_manager;
use crate::config::load_config;
use crate::core::matcher::{FileDiscovery, MatchConfig, MatchEngine, MediaFileType};
use crate::core::parallel::{
    FileProcessingTask, ProcessingOperation, Task, TaskResult, TaskScheduler,
};
use crate::services::ai::{AIClientFactory, AIProvider};
use indicatif::ProgressDrawTarget;

/// 執行 Match 命令，支援 Dry-run 與實際操作，並允許注入 AI 服務以便測試
pub async fn execute(args: MatchArgs) -> Result<()> {
    // 載入配置與建立 AI 客戶端
    let config = load_config()?;
    // 建立 AI 客戶端，根據配置自動選擇提供商與端點
    let ai_client = AIClientFactory::create_client(&config.ai)?;
    execute_with_client(args, ai_client).await
}

/// 執行 Match 流程，支援 Dry-run 與實際操作，AI 客戶端由外部注入
pub async fn execute_with_client(args: MatchArgs, ai_client: Box<dyn AIProvider>) -> Result<()> {
    // 載入配置並初始化匹配引擎
    let config = load_config()?;
    let match_config = MatchConfig {
        confidence_threshold: args.confidence as f32 / 100.0,
        max_sample_length: config.ai.max_sample_length,
        // 永遠進行內容分析，以便 Dry-run 時也能產生並快取結果
        enable_content_analysis: true,
        backup_enabled: args.backup || config.general.backup_enabled,
    };
    let engine = MatchEngine::new(ai_client, match_config);

    // 執行匹配運算
    let operations = engine.match_files(&args.path, args.recursive).await?;

    // 顯示對映結果表格
    display_match_results(&operations, args.dry_run);

    if args.dry_run {
        engine
            .save_cache(&args.path, args.recursive, &operations)
            .await?;
    } else {
        // 執行檔案操作
        engine.execute_operations(&operations, args.dry_run).await?;
    }
    Ok(())
}

/// Execute parallel match over multiple files using the parallel processing system
pub async fn execute_parallel_match(
    directory: &std::path::Path,
    recursive: bool,
    output: Option<&std::path::Path>,
) -> Result<()> {
    init_config_manager()?;
    let scheduler = TaskScheduler::new()?;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(directory, recursive)?;
    let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
    for f in files
        .iter()
        .filter(|f| matches!(f.file_type, MediaFileType::Video))
    {
        let task = Box::new(FileProcessingTask {
            input_path: f.path.clone(),
            output_path: output.map(|p| p.to_path_buf()),
            operation: ProcessingOperation::MatchFiles { recursive },
        });
        tasks.push(task);
    }
    if tasks.is_empty() {
        println!("未找到需要處理的影片檔案");
        return Ok(());
    }
    println!("準備並行處理 {} 個檔案", tasks.len());
    println!("最大並行數: {}", scheduler.get_active_workers());
    let progress_bar = {
        let pb = create_progress_bar(tasks.len());
        // 根據配置決定是否顯示進度條
        if let Ok(cfg) = load_config() {
            if !cfg.general.enable_progress_bar {
                pb.set_draw_target(ProgressDrawTarget::hidden());
            }
        }
        pb
    };
    let results = monitor_batch_execution(&scheduler, tasks, &progress_bar).await?;
    let (mut ok, mut failed, mut partial) = (0, 0, 0);
    for r in &results {
        match r {
            TaskResult::Success(_) => ok += 1,
            TaskResult::Failed(_) | TaskResult::Cancelled => failed += 1,
            TaskResult::PartialSuccess(_, _) => partial += 1,
        }
    }
    println!("\n處理完成統計:");
    println!("  ✓ 成功: {} 個檔案", ok);
    if partial > 0 {
        println!("  ⚠ 部分成功: {} 個檔案", partial);
    }
    if failed > 0 {
        println!("  ✗ 失敗: {} 個檔案", failed);
        for (i, r) in results.iter().enumerate() {
            if matches!(r, TaskResult::Failed(_)) {
                println!("  失敗詳情 {}: {}", i + 1, r);
            }
        }
    }
    Ok(())
}

async fn monitor_batch_execution(
    scheduler: &TaskScheduler,
    tasks: Vec<Box<dyn Task + Send + Sync>>,
    progress_bar: &indicatif::ProgressBar,
) -> Result<Vec<TaskResult>> {
    use tokio::time::{Duration, interval};
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|t| {
            let s = scheduler.clone();
            tokio::spawn(async move { s.submit_task(t).await })
        })
        .collect();
    let mut ticker = interval(Duration::from_millis(500));
    let mut completed = 0;
    let total = handles.len();
    let mut results = Vec::new();
    for mut h in handles {
        loop {
            tokio::select! {
                res = &mut h => {
                    match res {
                        Ok(Ok(r)) => results.push(r),
                        Ok(Err(_)) => results.push(TaskResult::Failed("任務執行錯誤".into())),
                        Err(_) => results.push(TaskResult::Cancelled),
                    }
                    completed += 1;
                    progress_bar.set_position(completed);
                    break;
                }
                _ = ticker.tick() => {
                    let active = scheduler.list_active_tasks().len();
                    let queued = scheduler.get_queue_size();
                    progress_bar.set_message(format!("執行中: {} | 佇列: {} | 已完成: {}/{}", active, queued, completed, total));
                }
            }
        }
    }
    progress_bar.finish_with_message("所有任務已完成");
    Ok(results)
}

fn create_progress_bar(total: usize) -> indicatif::ProgressBar {
    use indicatif::ProgressStyle;
    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

#[cfg(test)]
mod tests {
    use super::execute_with_client;
    use crate::cli::MatchArgs;
    use crate::config::init_config_manager;
    use crate::services::ai::{
        AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
    };
    use async_trait::async_trait;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    struct DummyAI;
    #[async_trait]
    impl AIProvider for DummyAI {
        async fn analyze_content(&self, _req: AnalysisRequest) -> crate::Result<MatchResult> {
            Ok(MatchResult {
                matches: Vec::new(),
                confidence: 0.0,
                reasoning: String::new(),
            })
        }
        async fn verify_match(&self, _req: VerificationRequest) -> crate::Result<ConfidenceScore> {
            panic!("verify_match should not be called in dry-run test");
        }
    }

    /// Dry-run 模式下應建立快取檔案，且不實際執行任何檔案操作
    #[tokio::test]
    async fn dry_run_creates_cache_and_skips_execute_operations() -> crate::Result<()> {
        // 建立臨時媒體資料夾並放入示意影片與字幕檔
        let media_dir = tempdir()?;
        let media_path = media_dir.path().join("media");
        fs::create_dir_all(&media_path)?;
        let video = media_path.join("video.mkv");
        let subtitle = media_path.join("subtitle.ass");
        fs::write(&video, b"dummy")?;
        fs::write(&subtitle, b"dummy")?;

        // 指定快取路徑到臨時資料夾
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", media_dir.path());
        }
        // 初始化配置管理器，以便使用新系統載入默認配置
        init_config_manager()?;

        // 確認尚未產生快取檔案
        let cache_path = dirs::config_dir()
            .unwrap()
            .join("subx")
            .join("match_cache.json");
        assert!(!cache_path.exists(), "測試開始時不應存在快取檔案");

        // 執行 dry-run
        let args = MatchArgs {
            path: PathBuf::from(&media_path),
            dry_run: true,
            recursive: false,
            confidence: 80,
            backup: false,
        };
        execute_with_client(args, Box::new(DummyAI)).await?;

        // 驗證已建立快取檔案，且原始檔案未被移動或刪除
        assert!(cache_path.exists(), "dry_run 後應建立快取檔案");
        assert!(
            video.exists(),
            "dry_run 不應執行 execute_operations，影片檔仍須存在"
        );
        assert!(
            subtitle.exists(),
            "dry_run 不應執行 execute_operations，字幕檔仍須存在"
        );
        Ok(())
    }
}
