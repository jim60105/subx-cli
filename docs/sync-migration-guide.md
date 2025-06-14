# SubX Sync Architecture Migration Guide

This guide helps you migrate from the legacy sync command to the new multi-method sync architecture introduced in SubX v0.6.0.

## Overview of Changes

### New Sync Methods

SubX now supports three main synchronization methods:

1. **OpenAI Whisper API** - Cloud-based speech recognition (new enhanced version)
2. **Local VAD** - On-device Voice Activity Detection (improved algorithm)
3. **Manual Offset** - User-specified time adjustment

### Breaking Changes

#### Command Line Interface

**Old CLI (v0.5.x and earlier):**
```bash
# Legacy sync command
subx sync audio.wav subtitle.srt --method correlation
subx sync audio.wav subtitle.srt --whisper-api --api-key sk-xxx
subx sync audio.wav subtitle.srt --vad-threshold 0.8
```

**New CLI (v0.6.0+):**
```bash
# New sync command with method selection
subx sync audio.wav subtitle.srt --method whisper
subx sync audio.wav subtitle.srt --method vad --vad-sensitivity 0.8
subx sync audio.wav subtitle.srt --method manual --manual-offset 2.5

# Advanced options
subx sync audio.wav subtitle.srt --method whisper \
    --whisper-model whisper-1 \
    --whisper-language zh \
    --whisper-fallback-to-vad

# Batch processing
subx sync --batch input_dir/ --output output_dir/ --method auto
```

#### Configuration File Changes

**Old Configuration (v0.5.x):**
```toml
[sync]
correlation_threshold = 0.8
dialogue_detection_threshold = 0.6
min_dialogue_duration_ms = 500
enable_dialogue_detection = true
audio_sample_rate = 44100
auto_detect_sample_rate = true
```

**New Configuration (v0.6.0+):**
```toml
[sync]
default_method = "whisper"
analysis_window_seconds = 30
max_offset_seconds = 60.0

[sync.whisper]
enabled = true
model = "whisper-1"
language = "auto"
temperature = 0.0
timeout_seconds = 30
max_retries = 3
retry_delay_ms = 1000
fallback_to_vad = true
min_confidence_threshold = 0.7

[sync.vad]
enabled = true
sensitivity = 0.75
chunk_size = 512
sample_rate = 16000
padding_chunks = 3
min_speech_duration_ms = 100
speech_merge_gap_ms = 200
```

### Migration Steps

#### Step 1: Update CLI Commands

Replace your existing sync commands with the new method-based syntax:

| Old Command | New Command |
|-------------|-------------|
| `subx sync audio.wav subtitle.srt` | `subx sync audio.wav subtitle.srt --method auto` |
| `subx sync audio.wav subtitle.srt --whisper-api` | `subx sync audio.wav subtitle.srt --method whisper` |
| `subx sync audio.wav subtitle.srt --vad` | `subx sync audio.wav subtitle.srt --method vad` |

#### Step 2: Update Configuration Files

1. **Backup your existing configuration:**
   ```bash
   cp ~/.config/subx/config.toml ~/.config/subx/config.toml.backup
   ```

2. **Update sync section:**
   - Remove deprecated fields: `correlation_threshold`, `dialogue_detection_threshold`, etc.
   - Add new method-specific configurations
   - Set appropriate default method

3. **Example migration:**
   ```bash
   # Generate new default configuration
   subx config reset
   
   # Restore your custom settings
   subx config set ai.api_key "your-api-key"
   subx config set sync.default_method "whisper"
   subx config set sync.whisper.language "zh"
   ```

#### Step 3: Test New Functionality

1. **Test with default method:**
   ```bash
   subx sync test_audio.wav test_subtitle.srt
   ```

2. **Test specific methods:**
   ```bash
   # Test Whisper API
   subx sync test_audio.wav test_subtitle.srt --method whisper
   
   # Test local VAD
   subx sync test_audio.wav test_subtitle.srt --method vad
   
   # Test manual offset
   subx sync test_audio.wav test_subtitle.srt --method manual --manual-offset 1.5
   ```

3. **Test batch processing:**
   ```bash
   subx sync --batch input_folder/ --output output_folder/ --method auto
   ```

### Parameter Mapping

#### Whisper-related Parameters

| Old Parameter | New Parameter | Description |
|---------------|---------------|-------------|
| `--whisper-api` | `--method whisper` | Enable Whisper method |
| `--api-key` | Set via config or env | API key configuration |
| N/A | `--whisper-model` | Specify Whisper model |
| N/A | `--whisper-language` | Force language detection |
| N/A | `--whisper-temperature` | Control randomness |
| N/A | `--whisper-fallback-to-vad` | Enable fallback |

#### VAD-related Parameters

| Old Parameter | New Parameter | Description |
|---------------|---------------|-------------|
| `--vad-threshold` | `--vad-sensitivity` | Speech detection sensitivity |
| N/A | `--vad-chunk-size` | Audio processing chunk size |
| N/A | `--vad-sample-rate` | Processing sample rate |
| N/A | `--vad-padding` | Speech padding chunks |

#### General Parameters

| Old Parameter | New Parameter | Description |
|---------------|---------------|-------------|
| `--output` | `--output` | Output file path (unchanged) |
| N/A | `--window` | Analysis window duration |
| N/A | `--batch` | Batch processing mode |
| N/A | `--method` | Sync method selection |

### Environment Variables

#### Old Environment Variables (deprecated)
```bash
export SUBX_CORRELATION_THRESHOLD=0.8
export SUBX_DIALOGUE_DETECTION_THRESHOLD=0.6
export SUBX_ENABLE_DIALOGUE_DETECTION=true
```

#### New Environment Variables
```bash
# Basic sync settings
export SUBX_SYNC_DEFAULT_METHOD=whisper
export SUBX_SYNC_ANALYSIS_WINDOW_SECONDS=30
export SUBX_SYNC_MAX_OFFSET_SECONDS=60.0

# Whisper settings
export SUBX_SYNC_WHISPER_ENABLED=true
export SUBX_SYNC_WHISPER_MODEL=whisper-1
export SUBX_SYNC_WHISPER_LANGUAGE=auto
export SUBX_SYNC_WHISPER_FALLBACK_TO_VAD=true

# VAD settings
export SUBX_SYNC_VAD_ENABLED=true
export SUBX_SYNC_VAD_SENSITIVITY=0.75
export SUBX_SYNC_VAD_CHUNK_SIZE=512
```

### Performance Considerations

#### New Architecture Benefits

1. **Better Accuracy**: Multi-method approach with fallback mechanisms
2. **Flexible Configuration**: Fine-tune each method independently
3. **Batch Processing**: Process multiple files efficiently
4. **Improved Error Handling**: Better error reporting and recovery

#### Migration Performance Tips

1. **Method Selection**: Choose the right method for your content
   - **Whisper**: Best for clear speech, multiple languages
   - **VAD**: Good for consistent audio quality, faster processing
   - **Auto**: Intelligent method selection based on content

2. **Configuration Optimization**: Tune parameters for your use case
   - Increase `analysis_window_seconds` for complex audio
   - Adjust `vad_sensitivity` for noisy environments
   - Use `whisper_fallback_to_vad` for reliability

### Troubleshooting

#### Common Migration Issues

1. **Command not found errors**
   ```bash
   # Old: subx sync --whisper-api
   # Error: unknown flag: --whisper-api
   # Fix: subx sync --method whisper
   ```

2. **Configuration parsing errors**
   ```bash
   # Error: unknown configuration key 'correlation_threshold'
   # Fix: Remove deprecated configuration keys
   subx config reset  # Reset to defaults and reconfigure
   ```

3. **Performance regression**
   ```bash
   # Try different methods to find the best performance
   subx sync --method vad  # Faster local processing
   subx sync --method whisper --whisper-timeout-seconds 60  # More time for accuracy
   ```

### Compatibility Notes

#### Backward Compatibility

- **Configuration**: Old configuration keys are ignored (not errors)
- **CLI**: Old CLI flags are rejected with helpful error messages
- **Output Format**: Subtitle output format remains unchanged

#### Deprecated Features

The following features are deprecated and will be removed in future versions:
- Legacy correlation-based sync method
- Old configuration keys (correlation_threshold, etc.)
- Environment variables with old naming

### Getting Help

If you encounter migration issues:

1. **Check the logs**: Use `--verbose` flag for detailed information
2. **Reset configuration**: Start fresh with `subx config reset`
3. **Test incrementally**: Try each method individually
4. **Report issues**: Create GitHub issues with migration problems

### Examples

#### Complete Migration Example

**Before (v0.5.x):**
```bash
# Configuration
export SUBX_CORRELATION_THRESHOLD=0.9
export SUBX_ENABLE_DIALOGUE_DETECTION=true

# Command
subx sync audio.wav subtitle.srt --vad-threshold 0.8
```

**After (v0.6.0+):**
```bash
# Configuration
export SUBX_SYNC_DEFAULT_METHOD=vad
export SUBX_SYNC_VAD_SENSITIVITY=0.8
export SUBX_SYNC_ANALYSIS_WINDOW_SECONDS=45

# Command
subx sync audio.wav subtitle.srt --method vad --vad-sensitivity 0.8
```

This migration guide should help you transition smoothly to the new sync architecture. The new system provides more flexibility and better accuracy while maintaining the core functionality you expect from SubX.
