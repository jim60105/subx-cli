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

    /// 一般錯誤
    #[error("未知錯誤: {0}")]
    Other(#[from] anyhow::Error),
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

    /// 建立文件匹配錯誤
    pub fn file_matching<S: Into<String>>(message: S) -> Self {
        SubXError::FileMatching {
            message: message.into(),
        }
    }
}
