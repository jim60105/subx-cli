// src/cli/ui.rs
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
