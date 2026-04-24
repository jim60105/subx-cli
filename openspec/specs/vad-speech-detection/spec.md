# VAD Speech Detection

## Purpose

Provide local Voice Activity Detection (VAD) so SubX can identify speech segments in video or audio files without transmitting data to external services, and expose a first-speech offset that the Timeline Sync engine uses to align subtitles. Implemented in `src/services/vad/` (`mod.rs`, `audio_loader.rs`, `audio_processor.rs`, `detector.rs`, `resample.rs`, `sync_detector.rs`) on top of the `voice_activity_detector` crate (Silero VAD V5 model) and the `symphonia` decoding stack.

## Requirements

### Requirement: Silero VAD V5 Model Loading

The system SHALL perform voice activity detection using the Silero VAD V5 model bundled with the `voice_activity_detector` crate, without requiring any runtime download, external network access, or on-disk model file managed by SubX itself. `LocalVadDetector::new` SHALL return an error (not panic) if the underlying audio processor cannot be initialized, and `VoiceActivityDetector::builder().build()` failures SHALL be converted into `SubXError::AudioProcessing` with a message prefixed `Failed to create VAD:`. Implemented in `src/services/vad/detector.rs`.

#### Scenario: Detector construction never downloads a model
- **GIVEN** a process with network access disabled
- **WHEN** `LocalVadDetector::new(VadConfig::default())` is called
- **THEN** the call SHALL succeed without issuing any network request and SHALL return a ready-to-use detector

#### Scenario: Underlying VAD build failure is surfaced as AudioProcessing error
- **GIVEN** an unsupported configuration that causes the internal `VoiceActivityDetector::builder().build()` to fail
- **WHEN** `detect_speech_from_data` is invoked
- **THEN** the detector SHALL return `SubXError::AudioProcessing` whose message contains `Failed to create VAD:` and SHALL NOT panic

### Requirement: Audio Extraction From Video And Audio Containers

The system SHALL extract a PCM audio track from any container that `symphonia` can probe, using file extension as a format hint. `DirectAudioLoader` SHALL select the first track whose `codec_params.sample_rate` is known as the audio source, decode it into interleaved i16 samples, and populate `AudioInfo { sample_rate, channels, duration_seconds, total_samples }`. The integration test `tests/vad_integration_tests.rs` and the unit test in `src/services/vad/audio_loader.rs` exercise at least WAV and MP4 inputs; the module comments additionally document MKV and OGG support.

#### Scenario: Loading an MP4 video extracts its audio track
- **GIVEN** a path to an MP4 file containing an audio track
- **WHEN** `DirectAudioLoader::load_audio_samples` is called
- **THEN** the loader SHALL return a non-empty `Vec<i16>` and an `AudioInfo` whose `sample_rate > 0` and `total_samples > 0`

#### Scenario: Missing audio track is rejected
- **GIVEN** a media file whose tracks have no known `sample_rate`
- **WHEN** `DirectAudioLoader::load_audio_samples` is called
- **THEN** it SHALL return `SubXError::AudioProcessing` with message `No audio track found`

### Requirement: Multi-Channel Downmix And Sample-Rate Preservation

`VadAudioProcessor::load_and_prepare_audio_direct` SHALL preserve the file's original sample rate in the returned `AudioInfo` and, for multi-channel input, SHALL keep only the first channel by striding across the interleaved samples (`step_by(channels)`). Resampling SHALL NOT happen in this function. Implemented in `src/services/vad/audio_processor.rs` and verified by `tests/vad_audio_processor_tests.rs`.

#### Scenario: Stereo input yields mono output at original rate
- **GIVEN** a 44100 Hz two-channel WAV file
- **WHEN** `load_and_prepare_audio_direct` processes it
- **THEN** the returned `AudioData.info.sample_rate` SHALL equal 44100 and `AudioData.info.channels` SHALL equal 1

#### Scenario: Empty source file yields empty samples instead of error
- **GIVEN** a zero-byte audio file path
- **WHEN** `load_and_prepare_audio_direct` is called
- **THEN** it SHALL return `ProcessedAudioData` with `samples.is_empty()` and a default `AudioInfo { sample_rate: 16000, channels: 1, duration_seconds: 0.0, total_samples: 0 }`

### Requirement: Resampling To Model-Compatible Rate

When the loaded audio's sample rate is neither 8000 Hz nor 16000 Hz, the detector SHALL resample it to 16000 Hz before running VAD, using `resample::resample_to_target_rate` (FFT-based resampler backed by `rubato::FftFixedIn`). After resampling, `AudioInfo.sample_rate`, `AudioInfo.total_samples`, and `AudioInfo.duration_seconds` SHALL be updated to reflect the new buffer. A matching input and output sample rate SHALL take a zero-copy fast path. Implemented in `src/services/vad/resample.rs` and `src/services/vad/detector.rs` (lines 66–83).

#### Scenario: 44.1 kHz input is resampled to 16 kHz
- **GIVEN** a `ProcessedAudioData` whose `info.sample_rate == 44100`
- **WHEN** `LocalVadDetector::detect_speech_from_data` runs
- **THEN** after the resampling branch the audio SHALL have `info.sample_rate == 16000` and `info.total_samples == resampled.len()`

#### Scenario: Matching sample rate skips resampling
- **GIVEN** input at exactly 16000 Hz (or 8000 Hz)
- **WHEN** `detect_speech_from_data` runs
- **THEN** no resampling SHALL occur and the sample buffer SHALL be forwarded to the VAD unchanged

### Requirement: Chunked Detection With Silero-Compatible Chunk Size

The detector SHALL select a fixed VAD chunk size via `LocalVadDetector::calculate_chunk_size(sample_rate)`: 256 samples for 8000 Hz input and 512 samples for 16000 Hz input. Any other sample rate reaching this function SHALL cause a panic that explicitly names the unsupported rate, because resampling to 16 kHz is expected to have already occurred upstream. The VAD SHALL be built with the resolved `(sample_rate, chunk_size)` pair before processing.

#### Scenario: Canonical chunk sizes per model-supported rate
- **GIVEN** a constructed `LocalVadDetector`
- **WHEN** `calculate_chunk_size(8000)` and `calculate_chunk_size(16000)` are called
- **THEN** they SHALL return `256` and `512` respectively

### Requirement: Sensitivity Threshold Inversion

The `sync.vad.sensitivity` configuration value (range 0.0–1.0) SHALL be converted to the underlying VAD speech probability threshold as `threshold = 1.0 - sensitivity`, so that a higher sensitivity produces a lower threshold and therefore SHALL detect at least as many speech chunks as a lower sensitivity over the same audio. Implemented in `src/services/vad/detector.rs` and exercised by `tests/vad_detector_tests.rs`.

#### Scenario: Higher sensitivity yields at least as many segments
- **GIVEN** the same audio processed with `sensitivity = 0.1` and `sensitivity = 0.9`
- **WHEN** both detectors run
- **THEN** the segment count at 0.1 SHALL be less than or equal to the segment count at 0.9 (tolerating a difference of at most 1)

### Requirement: Minimum Speech Duration Filter

Speech segments whose `duration < sync.vad.min_speech_duration_ms / 1000.0` seconds SHALL be discarded both for mid-stream segments and for any trailing segment left open at end-of-audio. Only segments satisfying the minimum duration SHALL appear in `VadResult.speech_segments`. The default value is 300 ms (`VadConfig::default`).

#### Scenario: Sub-threshold segment is dropped
- **GIVEN** `min_speech_duration_ms = 300` and a candidate segment of 120 ms
- **WHEN** `detect_speech_segments` finalizes the segment
- **THEN** the segment SHALL NOT be appended to the result

### Requirement: Padding Chunks Around Detected Speech

The detector SHALL pass `sync.vad.padding_chunks` (default 3) as the `padding_chunks` argument to `voice_activity_detector::IteratorExt::label`, so that the requested number of non-speech chunks on either side of a positive detection are relabeled as speech. `sync.vad.padding_chunks` is a non-negative `u32`.

#### Scenario: Padding value is forwarded to the labeler
- **GIVEN** a `VadConfig` with `padding_chunks = 5`
- **WHEN** `detect_speech_segments` invokes `.label(&mut vad, vad_threshold, self.config.padding_chunks as usize)`
- **THEN** the labeler SHALL receive the value `5` for padding

### Requirement: First-Speech Offset Extraction

`VadSyncDetector::detect_sync_offset` SHALL compute a synchronization offset as `first_speech_start - first_subtitle_start_time` (in seconds), where `first_speech_start` is the start time of the first speech segment of duration ≥ 0.1 s, falling back to the first segment of any duration if none qualify. The returned `SyncResult` SHALL set `method_used = SyncMethod::LocalVad`, include `additional_info.first_speech_start`, `additional_info.expected_subtitle_start`, `additional_info.speech_segments_count`, `additional_info.audio_duration`, `additional_info.processing_time_ms`, and a `detected_segments` array. An analysis-window parameter > 0 SHALL crop the audio to the first `window × sample_rate` samples before detection. Implemented in `src/services/vad/sync_detector.rs`.

#### Scenario: Offset equals first-speech minus expected-start
- **GIVEN** a loaded audio with a first qualifying speech segment at 2.000 s and a subtitle whose first entry starts at 1.500 s
- **WHEN** `VadSyncDetector::detect_sync_offset(audio_path, subtitle, 0)` completes
- **THEN** `SyncResult.offset_seconds` SHALL equal 0.5 (within f32 rounding) and `additional_info.first_speech_start - additional_info.expected_subtitle_start == offset_seconds`

#### Scenario: Analysis window crops the audio
- **GIVEN** a 120-second audio file and `analysis_window_seconds = 30`
- **WHEN** `detect_sync_offset` runs
- **THEN** the VAD SHALL only process the first 30 seconds and `audio_info.duration_seconds` SHALL reflect the cropped length

### Requirement: Error Handling For Degenerate Inputs

The VAD pipeline SHALL return typed errors (never panic) for recoverable failures: empty post-decode sample buffers SHALL raise `SubXError::AudioProcessing("Audio data is empty")`; a subtitle with zero entries SHALL raise `SubXError::AudioProcessing` whose message contains `No subtitle entries found`; and a non-existent or unopenable audio path SHALL be surfaced as `SubXError::AudioProcessing` with the underlying I/O or codec message embedded. Exercised by `tests/vad_audio_processor_tests.rs`, `tests/vad_detector_tests.rs`, and `tests/vad_integration_tests.rs`.

#### Scenario: Empty samples reach the detector
- **GIVEN** `ProcessedAudioData { samples: vec![], .. }`
- **WHEN** `detect_speech_from_data` is awaited
- **THEN** it SHALL return `SubXError::AudioProcessing` with message `Audio data is empty`

#### Scenario: Empty subtitle rejected by sync detector
- **GIVEN** a `Subtitle` whose `entries` is empty
- **WHEN** `VadSyncDetector::detect_sync_offset` is awaited
- **THEN** it SHALL return an error whose message contains `No subtitle entries found`

#### Scenario: Non-existent audio file
- **GIVEN** a path that does not exist
- **WHEN** `VadAudioProcessor::load_and_prepare_audio_direct` is awaited
- **THEN** the call SHALL return a `SubXError` rather than panic
