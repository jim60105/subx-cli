use clap::Args;

/// 檔案編碼檢測參數
#[derive(Args, Debug)]
pub struct DetectEncodingArgs {
    /// 顯示詳細樣本文字
    #[arg(short, long)]
    pub verbose: bool,
    /// 要檢測的檔案路徑
    #[arg(required = true)]
    pub file_paths: Vec<String>,
}
