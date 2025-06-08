use thiserror::Error;

/// SubX 應用程式的主要錯誤類型
#[derive(Error, Debug)]
pub enum SubXError {
    /// IO 相關錯誤
    #[error("IO 錯誤: {0}")]
    Io(#[from] std::io::Error),

    /// 配置錯誤
    #[error("配置錯誤: {message}")]
    Config { message: String },

    /// 字幕格式錯誤
    #[error("字幕格式錯誤: {format} - {message}")]
    SubtitleFormat { format: String, message: String },

    /// AI 服務錯誤
    #[error("AI 服務錯誤: {0}")]
    AiService(String),

    /// 音訊處理錯誤
    #[error("音訊處理錯誤: {message}")]
    AudioProcessing { message: String },

    /// 文件匹配錯誤
    #[error("文件匹配錯誤: {message}")]
    FileMatching { message: String },
    /// 檔案已存在錯誤
    #[error("檔案已存在: {0}")]
    FileAlreadyExists(String),
    /// 檔案不存在錯誤
    #[error("檔案不存在: {0}")]
    FileNotFound(String),
    /// 無效的檔案名稱錯誤
    #[error("無效的檔案名稱: {0}")]
    InvalidFileName(String),
    /// 檔案操作失敗錯誤
    #[error("檔案操作失敗: {0}")]
    FileOperationFailed(String),

    /// 一般錯誤
    #[error("未知錯誤: {0}")]
    Other(#[from] anyhow::Error),
}

// 單元測試: SubXError 錯誤類型與輔助方法
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_config_error_creation() {
        let error = SubXError::config("測試配置錯誤");
        assert!(matches!(error, SubXError::Config { .. }));
        assert_eq!(error.to_string(), "配置錯誤: 測試配置錯誤");
    }

    #[test]
    fn test_subtitle_format_error_creation() {
        let error = SubXError::subtitle_format("SRT", "無效格式");
        assert!(matches!(error, SubXError::SubtitleFormat { .. }));
        let msg = error.to_string();
        assert!(msg.contains("SRT"));
        assert!(msg.contains("無效格式"));
    }

    #[test]
    fn test_audio_processing_error_creation() {
        let error = SubXError::audio_processing("音訊解碼失敗");
        assert!(matches!(error, SubXError::AudioProcessing { .. }));
        assert_eq!(error.to_string(), "音訊處理錯誤: 音訊解碼失敗");
    }

    #[test]
    fn test_file_matching_error_creation() {
        let error = SubXError::file_matching("匹配失敗");
        assert!(matches!(error, SubXError::FileMatching { .. }));
        assert_eq!(error.to_string(), "文件匹配錯誤: 匹配失敗");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "檔案不存在");
        let subx_error: SubXError = io_error.into();
        assert!(matches!(subx_error, SubXError::Io(_)));
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(SubXError::config("test").exit_code(), 2);
        assert_eq!(SubXError::subtitle_format("SRT", "test").exit_code(), 4);
        assert_eq!(SubXError::audio_processing("test").exit_code(), 5);
        assert_eq!(SubXError::file_matching("test").exit_code(), 6);
    }

    #[test]
    fn test_user_friendly_messages() {
        let config_error = SubXError::config("API 金鑰未設定");
        let message = config_error.user_friendly_message();
        assert!(message.contains("配置錯誤"));
        assert!(message.contains("subx config --help"));

        let ai_error = SubXError::ai_service("網路連接失敗".to_string());
        let message = ai_error.user_friendly_message();
        assert!(message.contains("AI 服務錯誤"));
        assert!(message.contains("檢查網路連接"));
    }
}

// 將 reqwest 錯誤轉換為 AI 服務錯誤
impl From<reqwest::Error> for SubXError {
    fn from(err: reqwest::Error) -> Self {
        SubXError::AiService(err.to_string())
    }
}

// 將檔案探索錯誤轉換為文件匹配錯誤
impl From<walkdir::Error> for SubXError {
    fn from(err: walkdir::Error) -> Self {
        SubXError::FileMatching {
            message: err.to_string(),
        }
    }
}
// 將 symphonia 錯誤轉換為音訊處理錯誤
impl From<symphonia::core::errors::Error> for SubXError {
    fn from(err: symphonia::core::errors::Error) -> Self {
        SubXError::audio_processing(err.to_string())
    }
}
/// SubX 應用程式的 Result 類型
pub type SubXResult<T> = Result<T, SubXError>;

impl SubXError {
    /// 建立配置錯誤
    pub fn config<S: Into<String>>(message: S) -> Self {
        SubXError::Config {
            message: message.into(),
        }
    }

    /// 建立字幕格式錯誤
    pub fn subtitle_format<S1, S2>(format: S1, message: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SubXError::SubtitleFormat {
            format: format.into(),
            message: message.into(),
        }
    }

    /// 建立音訊處理錯誤
    pub fn audio_processing<S: Into<String>>(message: S) -> Self {
        SubXError::AudioProcessing {
            message: message.into(),
        }
    }

    /// 建立 AI 服務錯誤
    pub fn ai_service<S: Into<String>>(message: S) -> Self {
        SubXError::AiService(message.into())
    }

    /// 建立文件匹配錯誤
    pub fn file_matching<S: Into<String>>(message: S) -> Self {
        SubXError::FileMatching {
            message: message.into(),
        }
    }
    /// 建立對話檢測失敗錯誤
    pub fn dialogue_detection_failed<S: Into<String>>(msg: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("對話檢測失敗: {}", msg.into()),
        }
    }
    /// 建立無效音訊格式錯誤
    pub fn invalid_audio_format<S: Into<String>>(format: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("不支援的音訊格式: {}", format.into()),
        }
    }
    /// 建立無效對話片段錯誤
    pub fn dialogue_segment_invalid<S: Into<String>>(reason: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("無效的對話片段: {}", reason.into()),
        }
    }
    /// 取得對應退出狀態碼
    pub fn exit_code(&self) -> i32 {
        match self {
            SubXError::Io(_) => 1,
            SubXError::Config { .. } => 2,
            SubXError::AiService(_) => 3,
            SubXError::SubtitleFormat { .. } => 4,
            SubXError::AudioProcessing { .. } => 5,
            SubXError::FileMatching { .. } => 6,
            _ => 1,
        }
    }

    /// 取得用戶友善的錯誤訊息
    pub fn user_friendly_message(&self) -> String {
        match self {
            SubXError::Io(e) => format!("檔案操作錯誤: {}", e),
            SubXError::Config { message } => {
                format!(
                    "配置錯誤: {}\n提示: 使用 'subx config --help' 查看配置說明",
                    message
                )
            }
            SubXError::AiService(msg) => {
                format!("AI 服務錯誤: {}\n提示: 檢查網路連接和 API 金鑰設定", msg)
            }
            SubXError::SubtitleFormat { message, .. } => {
                format!("字幕處理錯誤: {}\n提示: 檢查檔案格式和編碼", message)
            }
            SubXError::AudioProcessing { message } => {
                format!(
                    "音訊處理錯誤: {}\n提示: 確認影片檔案完整且格式支援",
                    message
                )
            }
            SubXError::FileMatching { message } => {
                format!("檔案匹配錯誤: {}\n提示: 檢查檔案路徑和格式", message)
            }
            SubXError::FileAlreadyExists(path) => format!("檔案已存在: {}", path),
            SubXError::FileNotFound(path) => format!("檔案不存在: {}", path),
            SubXError::InvalidFileName(name) => format!("無效的檔案名稱: {}", name),
            SubXError::FileOperationFailed(msg) => format!("檔案操作失敗: {}", msg),
            SubXError::Other(err) => {
                format!("未知錯誤: {}\n提示: 請回報此問題", err)
            }
        }
    }
}
