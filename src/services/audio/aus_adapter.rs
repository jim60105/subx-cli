//! Adapter module for the aus crate.

use crate::{Result, error::SubXError};
use aus::AudioFile;
use std::path::Path;

/// Adapter to convert SubX AudioData to aus AudioFile.
pub struct AusAdapter {
    sample_rate: u32,
}

impl AusAdapter {
    /// Create a new AusAdapter.
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Read an audio file as aus AudioFile.
    pub fn read_audio_file<P: AsRef<Path>>(&self, path: P) -> Result<AudioFile> {
        let path_ref = path.as_ref();
        let path_str = path_ref
            .to_str()
            .ok_or_else(|| SubXError::audio_processing("Failed to convert path to UTF-8 string"))?;
        aus::read(path_str).map_err(|e| {
            SubXError::audio_processing(format!("aus failed to read audio file: {:?}", e))
        })
    }

    /// Convert AudioFile to SubX-compatible AudioData.
    pub fn to_subx_audio_data(
        &self,
        _audio_file: &AudioFile,
    ) -> Result<crate::services::audio::AudioData> {
        // Conversion logic to be implemented in later stage.
        todo!("Will implement aus to AudioData conversion in a later stage.")
    }
}
