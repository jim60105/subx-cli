//! 遷移驗證工具

use std::env;
use std::path::PathBuf;
use subx_cli::services::audio::{AudioAnalyzer, AusAudioAnalyzer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("使用方式: {} <音訊檔案路徑>", args[0]);
        std::process::exit(1);
    }

    let audio_path = PathBuf::from(&args[1]);
    println!("正在驗證音訊處理遷移...");
    println!("音訊檔案: {:?}", audio_path);

    // 建立兩個分析器
    let legacy_analyzer = AudioAnalyzer::new(16000);
    let aus_analyzer = AusAudioAnalyzer::new(16000);

    // 舊版實作
    println!("\n=== 舊版實作 ===");
    let start = std::time::Instant::now();
    let legacy_envelope = legacy_analyzer.extract_envelope(&audio_path).await?;
    let legacy_time = start.elapsed();
    println!("處理時間: {:?}", legacy_time);
    println!("能量樣本數: {}", legacy_envelope.samples.len());
    println!("音訊長度: {:.2}s", legacy_envelope.duration);

    // aus 實作
    println!("\n=== aus 實作 ===");
    let start = std::time::Instant::now();
    let aus_envelope = aus_analyzer.extract_envelope_v2(&audio_path).await?;
    let aus_time = start.elapsed();
    println!("處理時間: {:?}", aus_time);
    println!("能量樣本數: {}", aus_envelope.samples.len());
    println!("音訊長度: {:.2}s", aus_envelope.duration);

    // 效能比較
    let speedup = legacy_time.as_secs_f64() / aus_time.as_secs_f64();
    println!("\n=== 比較結果 ===");
    println!("速度提升: {:.2}x", speedup);

    // 功能測試
    println!("\n=== 功能測試 ===");
    let audio_file = aus_analyzer.load_audio_file(&audio_path).await?;
    let features = aus_analyzer.analyze_audio_features(&audio_file).await?;
    println!("頻譜分析幀數: {}", features.frames.len());
    if let Some(first_frame) = features.frames.first() {
        println!("第一幀特徵:");
        println!("  頻譜質心: {:.2} Hz", first_frame.spectral_centroid);
        println!("  頻譜熵: {:.3}", first_frame.spectral_entropy);
        println!("  零交叉率: {:.3}", first_frame.zero_crossing_rate);
    }

    println!("\n遷移驗證完成！");
    Ok(())
}
