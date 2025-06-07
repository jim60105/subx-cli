// src/cli/ui.rs
use crate::cli::table::{create_match_table, MatchDisplayRow};
use crate::core::matcher::MatchOperation;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

/// 列印成功訊息
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// 列印錯誤訊息
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// 列印警告訊息
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// 建立進度條
pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap(),
    );
    pb
}

/// 顯示 AI API 呼叫使用統計資訊
pub fn display_ai_usage(usage: &crate::services::ai::AiUsageStats) {
    println!("🤖 AI API 呼叫詳情:");
    println!("   模型: {}", usage.model);
    println!("   Prompt tokens: {}", usage.prompt_tokens);
    println!("   Completion tokens: {}", usage.completion_tokens);
    println!("   Total tokens: {}", usage.total_tokens);
    println!();
}

/// 顯示檔案對映結果，支援 dry-run 預覽模式
pub fn display_match_results(results: &[MatchOperation], is_dry_run: bool) {
    if results.is_empty() {
        println!("{}", "沒有找到匹配的檔案對映".yellow());
        return;
    }

    println!("\n{}", "📋 檔案對映結果".bold().blue());
    if is_dry_run {
        println!("{}", "🔍 預覽模式 (不會實際修改檔案)".yellow());
    }
    println!();

    let rows: Vec<MatchDisplayRow> = results
        .iter()
        .map(|op| MatchDisplayRow {
            status: if is_dry_run {
                "🔍 預覽".yellow().to_string()
            } else {
                "✅ 完成".green().to_string()
            },
            video_file: truncate_path(&op.video_file.path.to_string_lossy(), 30),
            subtitle_file: truncate_path(&op.subtitle_file.path.to_string_lossy(), 30),
            new_name: truncate_path(&op.new_subtitle_name, 30),
        })
        .collect();

    println!("{}", create_match_table(rows));

    println!(
        "\n{}",
        format!("總共處理了 {} 個檔案對映", results.len()).bold()
    );
}

/// 路徑或檔名過長時進行截斷顯示
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_table_display() {
        let rows = vec![MatchDisplayRow {
            status: "✅ 完成".to_string(),
            video_file: "movie1.mp4".to_string(),
            subtitle_file: "subtitle1.srt".to_string(),
            new_name: "movie1.srt".to_string(),
        }];

        let table = create_match_table(rows);
        assert!(table.contains("movie1.mp4"));
    }

    #[test]
    fn test_path_truncation() {
        let long_path = "/very/long/path/to/some/movie/file.mp4";
        let truncated = truncate_path(long_path, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.starts_with("..."));
    }
}
