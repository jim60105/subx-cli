use tabled::settings::{Alignment, Modify, Style, object::Rows};
use tabled::{Table, Tabled};

/// 對映結果顯示列結構
#[derive(Tabled)]
pub struct MatchDisplayRow {
    #[tabled(rename = "狀態")]
    pub status: String,
    #[tabled(rename = "影片檔案")]
    pub video_file: String,
    #[tabled(rename = "字幕檔案")]
    pub subtitle_file: String,
    #[tabled(rename = "新檔名")]
    pub new_name: String,
}

/// 建立檔案對映結果表格字串
pub fn create_match_table(rows: Vec<MatchDisplayRow>) -> String {
    let mut table = Table::new(rows);
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    table.to_string()
}
