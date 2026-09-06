use subx_cli::cli::table::{MatchDisplayRow, create_match_table};
use subx_cli::cli::ui::create_progress_bar;

#[test]
fn test_table_formatting_cli() {
    // Characterised (B4 revival): MatchDisplayRow is now the two-column
    // Type/Path row shape; assert the headers and the cell content it renders.
    let rows = vec![MatchDisplayRow {
        file_type: "🔍 Preview Video 1".to_string(),
        file_path: "movie.mp4".to_string(),
    }];
    let table = create_match_table(rows);
    assert!(table.contains("Type"));
    assert!(table.contains("Path"));
    assert!(table.contains("🔍 Preview Video 1"));
    assert!(table.contains("movie.mp4"));
}

#[test]
fn test_progress_bar_total_length() {
    let pb = create_progress_bar(7);
    assert_eq!(pb.length(), Some(7));
}

// test_display_ai_usage_outputs was dropped under task 3.3/3.7's rule
// (B4 revival): display_ai_usage prints straight to the process stdout with
// no injectable writer seam (src/cli/ui.rs:309), so the only assertion the
// revived file could make was "does not panic" — a vacuous non-observation.
// The deletion and its lost (vacuous) coverage are recorded in the
// CHANGELOG; restore a real assertion when printing gains a capturable sink.
