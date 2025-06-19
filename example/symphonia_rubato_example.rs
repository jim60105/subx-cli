use std::fs::File;
use std::path::Path;

// Symphonia imports
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// Rubato imports
use rubato::{
    Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction,
};

// For WAV writing (you'll need to add hound to your Cargo.toml)
use hound::{WavSpec, WavWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_file = "input.mp3";
    let output_file = "output.wav";
    let target_sample_rate = 48000; // Target sample rate for resampling
    
    // Decode MP3 with Symphonia
    let decoded_audio = decode_audio_file(input_file)?;
    
    // Resample with Rubato
    let resampled_audio = resample_audio(
        decoded_audio.samples,
        decoded_audio.sample_rate,
        target_sample_rate,
        decoded_audio.channels,
    )?;
    
    // Write to WAV file
    write_wav_file(
        output_file,
        &resampled_audio,
        target_sample_rate,
        decoded_audio.channels,
    )?;
    
    println!("Successfully converted {} to {}", input_file, output_file);
    Ok(())
}

struct DecodedAudio {
    samples: Vec<Vec<f32>>, // Non-interleaved format required by Rubato
    sample_rate: u32,
    channels: usize,
}

fn decode_audio_file(path: &str) -> Result<DecodedAudio, Box<dyn std::error::Error>> {
    // Open the audio file
    let file = Box::new(File::open(Path::new(path))?);
    let mss = MediaSourceStream::new(file, Default::default());
    
    // Create format hint (can help with format detection)
    let mut hint = Hint::new();
    hint.with_extension("mp3"); // Hint that this is an MP3 file
    
    // Use default options
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();
    
    // Probe the file to determine the format
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)?;
    
    let mut format = probed.format;
    
    // Find the first audio track with a supported codec
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No supported audio tracks found")?;
    
    // Create decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)?;
    
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.unwrap().count();
    
    // Prepare storage for decoded samples (non-interleaved format)
    let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); channels];
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    
    // Decode loop
    loop {
        // Get next packet
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                // Handle format changes - for simplicity, we'll just break
                break;
            }
            Err(Error::IoError(_)) => break, // End of file
            Err(err) => return Err(err.into()),
        };
        
        // Skip packets from other tracks
        if packet.track_id() != track_id {
            continue;
        }
        
        // Decode the packet
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                // Create sample buffer on first decode
                if sample_buf.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }
                
                if let Some(buf) = &mut sample_buf {
                    // Copy decoded samples to the sample buffer
                    buf.copy_interleaved_ref(audio_buf);
                    
                    // Convert from interleaved to non-interleaved format
                    let samples = buf.samples();
                    for (i, &sample) in samples.iter().enumerate() {
                        let channel = i % channels;
                        all_samples[channel].push(sample);
                    }
                }
            }
            Err(Error::DecodeError(err)) => {
                // Skip decode errors
                eprintln!("Decode error: {}", err);
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    }
    
    Ok(DecodedAudio {
        samples: all_samples,
        sample_rate,
        channels,
    })
}

fn resample_audio(
    input_samples: Vec<Vec<f32>>,
    input_sample_rate: u32,
    output_sample_rate: u32,
    channels: usize,
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if input_sample_rate == output_sample_rate {
        // No resampling needed
        return Ok(input_samples);
    }
    
    let resample_ratio = output_sample_rate as f64 / input_sample_rate as f64;
    let chunk_size = 1024;
    
    // Configure Rubato resampler with high quality settings
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    
    // Create resampler
    let mut resampler = SincFixedIn::<f32>::new(
        resample_ratio,
        2.0, // Maximum allowed ratio change
        params,
        chunk_size,
        channels,
    )?;
    
    // Prepare output storage
    let expected_output_len = (input_samples[0].len() as f64 * resample_ratio) as usize;
    let mut output_samples: Vec<Vec<f32>> = vec![Vec::with_capacity(expected_output_len); channels];
    
    // Process audio in chunks
    let mut input_pos = 0;
    let input_len = input_samples[0].len();
    
    while input_pos < input_len {
        let frames_needed = resampler.input_frames_next();
        let frames_available = input_len - input_pos;
        
        if frames_available >= frames_needed {
            // Process full chunk
            let input_chunk: Vec<&[f32]> = input_samples
                .iter()
                .map(|ch| &ch[input_pos..input_pos + frames_needed])
                .collect();
            
            let output_chunk = resampler.process(&input_chunk, None)?;
            
            // Append to output
            for (out_ch, chunk_ch) in output_samples.iter_mut().zip(output_chunk.iter()) {
                out_ch.extend_from_slice(chunk_ch);
            }
            
            input_pos += frames_needed;
        } else {
            // Process remaining frames with partial processing
            let input_chunk: Vec<&[f32]> = input_samples
                .iter()
                .map(|ch| &ch[input_pos..])
                .collect();
            
            let output_chunk = resampler.process_partial(Some(&input_chunk), None)?;
            
            // Append to output
            for (out_ch, chunk_ch) in output_samples.iter_mut().zip(output_chunk.iter()) {
                out_ch.extend_from_slice(chunk_ch);
            }
            
            break;
        }
    }
    
    // Flush any remaining samples from the resampler
    loop {
        let output_chunk = resampler.process_partial(None, None)?;
        if output_chunk[0].is_empty() {
            break;
        }
        
        for (out_ch, chunk_ch) in output_samples.iter_mut().zip(output_chunk.iter()) {
            out_ch.extend_from_slice(chunk_ch);
        }
    }
    
    println!(
        "Resampled from {}Hz to {}Hz: {} -> {} samples",
        input_sample_rate,
        output_sample_rate,
        input_len,
        output_samples[0].len()
    );
    
    Ok(output_samples)
}

fn write_wav_file(
    path: &str,
    samples: &[Vec<f32>],
    sample_rate: u32,
    channels: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: channels as u16,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = WavWriter::create(path, spec)?;
    
    // Convert to interleaved format and write
    let num_samples = samples[0].len();
    for i in 0..num_samples {
        for ch in 0..channels {
            // Convert f32 to i16
            let sample_f32 = samples[ch][i];
            let sample_i16 = (sample_f32.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(sample_i16)?;
        }
    }
    
    writer.finalize()?;
    Ok(())
}