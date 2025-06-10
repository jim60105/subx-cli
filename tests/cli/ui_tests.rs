use subx_cli::cli::ui::{create_progress_bar, display_ai_usage};
use subx_cli::cli::table::{MatchDisplayRow, create_match_table};
use subx_cli::services::ai::AiUsageStats;

#[test]
fn test_table_formatting_cli() {
    let rows = vec![MatchDisplayRow {
        status: "🔍 Preview".to_string(),
        video_file: "檔案".to_string(),
        subtitle_file: "編碼".to_string(),
        new_name: "信心值".to_string(),
    }];
    let table = create_match_table(rows);
    assert!(table.contains("檔案"));
    assert!(table.contains("編碼"));
    assert!(table.contains("信心值"));
}

#[test]
fn test_progress_bar_total_length() {
    let pb = create_progress_bar(7);
    assert_eq!(pb.length(), 7);
}

#[test]
fn test_display_ai_usage_outputs() {
    let usage = AiUsageStats {
        model: "test-model".to_string(),
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
    };
    // Ensure display_ai_usage does not panic when printing
    display_ai_usage(&usage);
}
