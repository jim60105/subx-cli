use crate::config::WhisperConfig;
use crate::{Result, error::SubXError};
use reqwest::{Client, multipart::Form};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

/// OpenAI Whisper API 客戶端
pub struct WhisperApiClient {
    client: Client,
    api_key: String,
    base_url: String,
    config: WhisperConfig,
}

impl WhisperApiClient {
    /// 建立 Whisper API 客戶端
    pub fn new(api_key: String, base_url: String, config: WhisperConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds as u64))
            .build()
            .map_err(|e| SubXError::whisper_api(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            base_url,
            config,
        })
    }

    /// 轉錄音訊檔案並重試
    pub async fn transcribe(&self, audio_path: &Path) -> Result<WhisperResponse> {
        let mut retries = 0;
        let mut last_error = None;

        while retries <= self.config.max_retries {
            match self.try_transcribe(audio_path).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_error = Some(e);
                    if retries < self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                        retries += 1;
                        continue;
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| SubXError::whisper_api("Unknown Whisper API error")))
    }

    async fn try_transcribe(&self, audio_path: &Path) -> Result<WhisperResponse> {
        let file = File::open(audio_path).await.map_err(|e| {
            SubXError::audio_extraction(format!("Failed to open audio file: {}", e))
        })?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);

        let filename = audio_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "audio.wav".to_string());
        let mut form = Form::new()
            .text("model", self.config.model.clone())
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "word")
            .text("timestamp_granularities[]", "segment")
            .part(
                "file",
                reqwest::multipart::Part::stream(body)
                    .file_name(filename)
                    .mime_str("audio/wav")?,
            );

        if self.config.language != "auto" {
            form = form.text("language", self.config.language.clone());
        }
        if self.config.temperature > 0.0 {
            form = form.text("temperature", self.config.temperature.to_string());
        }

        let response = self
            .client
            .post(&format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| SubXError::whisper_api(format!("Whisper API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SubXError::whisper_api(format!(
                "Whisper API error {}: {}",
                status, text
            )));
        }

        let result: WhisperResponse = response.json().await.map_err(|e| {
            SubXError::whisper_api(format!("Failed to parse Whisper response: {}", e))
        })?;
        Ok(result)
    }
}

/// Whisper API 回應結構
#[derive(Debug, Deserialize)]
pub struct WhisperResponse {
    pub text: String,
    pub segments: Vec<WhisperSegment>,
    pub words: Option<Vec<WhisperWord>>,
}

/// Whisper API 時間段
#[derive(Debug, Deserialize)]
pub struct WhisperSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Whisper API 詞彙時間戳
#[derive(Debug, Deserialize)]
pub struct WhisperWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WhisperConfig;

    #[tokio::test]
    async fn test_whisper_client_creation() {
        let cfg = WhisperConfig::default();
        let client = WhisperApiClient::new("key".into(), "https://api.openai.com/v1".into(), cfg);
        assert!(client.is_ok());
    }
}
