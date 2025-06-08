# Product Backlog #16: 實作缺失功能模組

## 領域範圍
未實作功能開發、配置項目對應功能實作、系統功能完整性提升

## 背景描述

**更新日期**: 2025-06-08  
**架構狀況**: 基於統一配置管理系統 (Backlog #14 已完成)

根據統一配置系統實作完成後的分析，發現部分配置項目對應的功能尚未實作。目前 SubX 已完成：
- ✅ 統一配置管理系統 (`ConfigManager`)
- ✅ 檔案格式引擎 (`src/core/formats/`)
- ✅ 檔案匹配引擎 (`src/core/matcher/`)
- ✅ 語言檢測系統 (`src/core/language.rs`)
- ✅ AI 服務整合 (`src/services/ai/`)
- ✅ 基礎音訊同步引擎 (`src/core/sync/`)

需要實作的功能模組應整合到現有架構中，確保與統一配置系統的完整相容。

## 缺失功能清單

### 1. 對話檢測功能 (Dialogue Detection)
**相關配置**: `sync.dialogue_detection_threshold`, `sync.min_dialogue_duration_ms`
**目標模組**: `src/core/sync/dialogue.rs` (新增)

#### 功能描述
- 自動檢測音訊中的對話片段
- 識別語音活動和靜默期間
- 為現有同步引擎提供更精確的時間點
- 整合到統一配置系統，透過 `load_config()` 載入配置

#### 技術需求
- 音訊能量分析
- 語音活動檢測 (VAD)
- 對話片段分割
- 與現有 `SyncEngine` 整合

### 2. 平行處理系統 (Parallel Processing)
**相關配置**: `general.max_concurrent_jobs`
**目標模組**: `src/core/parallel/` (新增目錄)

#### 功能描述
- 支援多檔案並行處理
- 資源使用量控制
- 任務佇列管理
- 進度追蹤和錯誤處理
- 與現有命令系統整合

#### 技術需求
- 任務調度器
- 工作者執行緒池
- 任務狀態管理
- 資源限制控制

### 3. 音訊採樣率動態配置 (Dynamic Audio Sample Rate)
**相關配置**: `sync.audio_sample_rate`
**目標模組**: `src/services/audio/resampler.rs` (新增)

#### 功能描述
- 支援不同採樣率的音訊檔案
- 自動音訊重採樣
- 採樣率最佳化建議
- 整合到現有音訊處理服務

#### 技術需求
- 音訊重採樣演算法
- 採樣率檢測和轉換
- 音訊品質評估
- 與現有 `AudioAnalyzer` 整合

### 4. 檔案編碼自動檢測 (Automatic Encoding Detection)
**相關配置**: `formats.default_encoding`
**目標模組**: `src/core/formats/encoding.rs` (新增)

#### 功能描述
- 自動檢測字幕檔案編碼
- 支援多種字符編碼格式
- 編碼轉換和修復
- 整合到現有格式引擎

#### 技術需求
- 編碼檢測演算法
- 字符編碼轉換
- 編碼錯誤修復
- 與現有 `FormatManager` 整合

## 詳細實作計劃

### 模組 1: 對話檢測功能實作

#### 階段 1: 音訊分析基礎 (預估工時: 12 小時)

##### 1.1 建立對話檢測引擎
```rust
// src/core/sync/dialogue.rs
pub mod detector;
pub mod analyzer;
pub mod segment;

pub use detector::DialogueDetector;
pub use analyzer::{AudioAnalyzer, EnergyAnalyzer};
pub use segment::{DialogueSegment, SilenceSegment};
```

##### 1.2 實作音訊能量分析
```rust
// src/core/sync/dialogue/analyzer.rs
use std::collections::VecDeque;

pub struct EnergyAnalyzer {
    window_size: usize,
    hop_size: usize,
    threshold: f32,
    min_duration_ms: u64,
}

impl EnergyAnalyzer {
    pub fn new(threshold: f32, min_duration_ms: u64) -> Self {
        Self {
            window_size: 1024,
            hop_size: 512,
            threshold,
            min_duration_ms,
        }
    }
    
    pub fn analyze(&self, audio_data: &[f32], sample_rate: u32) -> Vec<DialogueSegment> {
        let mut segments = Vec::new();
        let mut energy_buffer = VecDeque::new();
        
        // 滑動視窗能量計算
        for (i, chunk) in audio_data.chunks(self.hop_size).enumerate() {
            let energy = self.calculate_energy(chunk);
            energy_buffer.push_back(energy);
            
            if energy_buffer.len() > self.window_size / self.hop_size {
                energy_buffer.pop_front();
            }
            
            // 檢測語音活動
            let is_speech = self.detect_speech(&energy_buffer);
            let timestamp = (i * self.hop_size) as f64 / sample_rate as f64;
            
            // 建立對話片段
            if is_speech {
                if let Some(last_segment) = segments.last_mut() {
                    if last_segment.is_speech {
                        last_segment.end_time = timestamp;
                    } else {
                        segments.push(DialogueSegment::new_speech(timestamp, timestamp));
                    }
                } else {
                    segments.push(DialogueSegment::new_speech(timestamp, timestamp));
                }
            }
        }
        
        // 過濾過短的片段
        self.filter_short_segments(segments)
    }
    
    fn calculate_energy(&self, chunk: &[f32]) -> f32 {
        chunk.iter().map(|&x| x * x).sum::<f32>() / chunk.len() as f32
    }
    
    fn detect_speech(&self, energy_buffer: &VecDeque<f32>) -> bool {
        let avg_energy: f32 = energy_buffer.iter().sum::<f32>() / energy_buffer.len() as f32;
        avg_energy > self.threshold
    }
    
    fn filter_short_segments(&self, segments: Vec<DialogueSegment>) -> Vec<DialogueSegment> {
        let min_duration = self.min_duration_ms as f64 / 1000.0;
        segments.into_iter()
            .filter(|segment| segment.duration() >= min_duration)
            .collect()
    }
}
```

##### 1.3 定義對話片段結構
```rust
// src/core/sync/dialogue/segment.rs
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub is_speech: bool,
    pub confidence: f32,
}

impl DialogueSegment {
    pub fn new_speech(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: true,
            confidence: 1.0,
        }
    }
    
    pub fn new_silence(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: false,
            confidence: 1.0,
        }
    }
    
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
    
    pub fn overlaps_with(&self, other: &DialogueSegment) -> bool {
        self.start_time < other.end_time && self.end_time > other.start_time
    }
}

#[derive(Debug, Clone)]
pub struct SilenceSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
}

impl SilenceSegment {
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            duration: end - start,
        }
    }
}
```

#### 階段 2: 對話檢測器整合 (預估工時: 8 小時)

##### 2.1 實作對話檢測器
```rust
// src/core/sync/dialogue/detector.rs
use crate::config::{load_config, SyncConfig};
use crate::core::sync::dialogue::{EnergyAnalyzer, DialogueSegment};
use crate::Result;
use std::path::Path;

pub struct DialogueDetector {
    energy_analyzer: EnergyAnalyzer,
    config: SyncConfig,
}

impl DialogueDetector {
    pub fn new() -> Result<Self> {
        let config = load_config()?.sync;
        let energy_analyzer = EnergyAnalyzer::new(
            config.dialogue_detection_threshold,
            config.min_dialogue_duration_ms,
        );
        
        Ok(Self {
            energy_analyzer,
            config,
        })
    }
    
    pub async fn detect_dialogue(&self, audio_path: &Path) -> Result<Vec<DialogueSegment>> {
        // 載入音訊檔案
        let audio_data = self.load_audio(audio_path).await?;
        
        // 執行對話檢測
        let segments = self.energy_analyzer.analyze(&audio_data.samples, audio_data.sample_rate);
        
        // 後處理和最佳化
        let optimized_segments = self.optimize_segments(segments);
        
        Ok(optimized_segments)
    }
    
    async fn load_audio(&self, audio_path: &Path) -> Result<AudioData> {
        // 使用現有的 AudioAnalyzer 或實作新的音訊載入器
        use crate::services::audio::AudioAnalyzer;
        let analyzer = AudioAnalyzer::new()?;
        analyzer.load_audio_file(audio_path).await
    }
    
    fn optimize_segments(&self, segments: Vec<DialogueSegment>) -> Vec<DialogueSegment> {
        // 合併相鄰的語音片段
        let mut optimized = Vec::new();
        let mut current_segment: Option<DialogueSegment> = None;
        
        for segment in segments {
            match current_segment.take() {
                Some(mut prev) if prev.is_speech && segment.is_speech => {
                    // 檢查間隔是否足夠小以進行合併
                    if segment.start_time - prev.end_time < 0.5 { // 0.5 秒間隔
                        prev.end_time = segment.end_time;
                        current_segment = Some(prev);
                    } else {
                        optimized.push(prev);
                        current_segment = Some(segment);
                    }
                }
                Some(prev) => {
                    optimized.push(prev);
                    current_segment = Some(segment);
                }
                None => {
                    current_segment = Some(segment);
                }
            }
        }
        
        if let Some(segment) = current_segment {
            optimized.push(segment);
        }
        
        optimized
    }
}
```

##### 2.2 整合到現有同步引擎
```rust
// 修改 src/core/sync/engine.rs
use crate::core::sync::dialogue::DialogueDetector;

impl SyncEngine {
    pub async fn sync_with_dialogue_detection(
        &self,
        subtitle_path: &Path,
        audio_path: &Path,
    ) -> Result<Vec<SyncResult>> {
        // 檢測對話片段
        let dialogue_detector = DialogueDetector::new()?;
        let dialogue_segments = dialogue_detector.detect_dialogue(audio_path).await?;
        
        // 載入字幕
        let subtitles = self.load_subtitles(subtitle_path).await?;
        
        // 使用對話片段輔助同步
        let sync_results = self.sync_with_dialogue_hints(&subtitles, &dialogue_segments).await?;
        
        Ok(sync_results)
    }
    
    async fn sync_with_dialogue_hints(
        &self,
        subtitles: &[Subtitle],
        dialogue_segments: &[DialogueSegment],
    ) -> Result<Vec<SyncResult>> {
        let mut results = Vec::new();
        
        for subtitle in subtitles {
            // 尋找最匹配的對話片段
            let best_match = self.find_best_dialogue_match(subtitle, dialogue_segments);
            
            if let Some(segment) = best_match {
                // 根據對話片段調整字幕時間
                let adjusted_subtitle = self.adjust_subtitle_timing(subtitle, &segment);
                results.push(SyncResult::Adjusted(adjusted_subtitle));
            } else {
                // 使用原有的同步邏輯
                let correlation_result = self.correlate_subtitle(subtitle).await?;
                results.push(correlation_result);
            }
        }
        
        Ok(results)
    }
    
    fn find_best_dialogue_match(
        &self,
        subtitle: &Subtitle,
        dialogue_segments: &[DialogueSegment],
    ) -> Option<DialogueSegment> {
        let subtitle_duration = subtitle.end_time - subtitle.start_time;
        
        dialogue_segments
            .iter()
            .filter(|segment| segment.is_speech)
            .min_by_key(|segment| {
                // 計算時間和長度的相似度
                let time_diff = (segment.start_time - subtitle.start_time).abs();
                let duration_diff = (segment.duration() - subtitle_duration).abs();
                
                // 組合得分 (越小越好)
                ((time_diff + duration_diff) * 1000.0) as i64
            })
            .cloned()
    }
}
```

### 模組 2: 平行處理系統實作

#### 階段 1: 任務調度器建立 (預估工時: 10 小時)

##### 2.1 建立任務調度系統
```rust
// src/core/parallel/mod.rs
pub mod scheduler;
pub mod worker;
pub mod task;
pub mod pool;

pub use scheduler::TaskScheduler;
pub use worker::{Worker, WorkerPool};
pub use task::{Task, TaskResult, TaskStatus};
```

##### 2.2 實作任務調度器
```rust
// src/core/parallel/scheduler.rs
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use tokio::sync::{mpsc, oneshot, Semaphore};
use crate::config::load_config;
use crate::Result;

pub struct TaskScheduler {
    task_queue: Arc<Mutex<VecDeque<PendingTask>>>,
    worker_pool: WorkerPool,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

struct PendingTask {
    task: Box<dyn Task + Send + Sync>,
    result_sender: oneshot::Sender<TaskResult>,
}

impl TaskScheduler {
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let max_concurrent_jobs = config.general.max_concurrent_jobs as usize;
        
        let worker_pool = WorkerPool::new(max_concurrent_jobs);
        let semaphore = Arc::new(Semaphore::new(max_concurrent_jobs));
        
        Ok(Self {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            worker_pool,
            semaphore,
            max_concurrent: max_concurrent_jobs,
        })
    }
    
    pub async fn submit_task(&self, task: Box<dyn Task + Send + Sync>) -> TaskResult {
        let (tx, rx) = oneshot::channel();
        
        // 取得執行許可
        let permit = self.semaphore.acquire().await.unwrap();
        
        // 提交任務到工作者池
        let task_handle = self.worker_pool.execute(task).await;
        
        // 等待任務完成
        let result = task_handle.await;
        
        // 釋放許可
        drop(permit);
        
        result
    }
    
    pub async fn submit_batch_tasks(&self, tasks: Vec<Box<dyn Task + Send + Sync>>) -> Vec<TaskResult> {
        let mut handles = Vec::new();
        
        for task in tasks {
            let handle = tokio::spawn({
                let scheduler = self.clone();
                async move {
                    scheduler.submit_task(task).await
                }
            });
            handles.push(handle);
        }
        
        // 等待所有任務完成
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        
        results
    }
    
    pub fn get_queue_size(&self) -> usize {
        self.task_queue.lock().unwrap().len()
    }
    
    pub fn get_active_workers(&self) -> usize {
        self.max_concurrent - self.semaphore.available_permits()
    }
}

impl Clone for TaskScheduler {
    fn clone(&self) -> Self {
        Self {
            task_queue: Arc::clone(&self.task_queue),
            worker_pool: self.worker_pool.clone(),
            semaphore: Arc::clone(&self.semaphore),
            max_concurrent: self.max_concurrent,
        }
    }
}
```

##### 2.3 定義任務介面
```rust
// src/core/parallel/task.rs
use async_trait::async_trait;
use std::fmt;

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self) -> TaskResult;
    fn task_type(&self) -> &'static str;
    fn task_id(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    Success(String),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed(TaskResult),
    Failed(String),
}

impl fmt::Display for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskResult::Success(msg) => write!(f, "Success: {}", msg),
            TaskResult::Failed(msg) => write!(f, "Failed: {}", msg),
            TaskResult::Cancelled => write!(f, "Cancelled"),
        }
    }
}

// 具體任務實作範例
pub struct FileProcessingTask {
    pub input_path: std::path::PathBuf,
    pub output_path: std::path::PathBuf,
    pub operation: ProcessingOperation,
}

#[derive(Debug, Clone)]
pub enum ProcessingOperation {
    ConvertFormat,
    SyncSubtitle,
    MatchFiles,
}

#[async_trait]
impl Task for FileProcessingTask {
    async fn execute(&self) -> TaskResult {
        match self.operation {
            ProcessingOperation::ConvertFormat => {
                // 實作格式轉換邏輯
                TaskResult::Success(format!("Converted {} to {}", 
                    self.input_path.display(), 
                    self.output_path.display()))
            }
            ProcessingOperation::SyncSubtitle => {
                // 實作字幕同步邏輯
                TaskResult::Success(format!("Synced subtitle for {}", 
                    self.input_path.display()))
            }
            ProcessingOperation::MatchFiles => {
                // 實作檔案匹配邏輯
                TaskResult::Success(format!("Matched files in {}", 
                    self.input_path.display()))
            }
        }
    }
    
    fn task_type(&self) -> &'static str {
        match self.operation {
            ProcessingOperation::ConvertFormat => "convert",
            ProcessingOperation::SyncSubtitle => "sync",
            ProcessingOperation::MatchFiles => "match",
        }
    }
    
    fn task_id(&self) -> String {
        format!("{}_{}", self.task_type(), self.input_path.display())
    }
}
```

#### 階段 2: 工作者池實作 (預估工時: 8 小時)

##### 2.4 實作工作者池
```rust
// src/core/parallel/worker.rs
use tokio::task::JoinHandle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct WorkerPool {
    workers: Arc<Mutex<HashMap<Uuid, JoinHandle<TaskResult>>>>,
    max_workers: usize,
}

impl WorkerPool {
    pub fn new(max_workers: usize) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            max_workers,
        }
    }
    
    pub async fn execute(&self, task: Box<dyn Task + Send + Sync>) -> JoinHandle<TaskResult> {
        let worker_id = Uuid::new_v4();
        
        let handle = tokio::spawn(async move {
            let result = task.execute().await;
            result
        });
        
        // 存儲工作者句柄
        self.workers.lock().unwrap().insert(worker_id, handle);
        
        // 返回句柄的複製版本（實際上需要更複雜的處理）
        tokio::spawn(async move {
            TaskResult::Success("Task completed".to_string())
        })
    }
    
    pub fn get_active_count(&self) -> usize {
        self.workers.lock().unwrap().len()
    }
    
    pub async fn shutdown(&self) {
        let workers = {
            let mut workers_guard = self.workers.lock().unwrap();
            std::mem::take(&mut *workers_guard)
        };
        
        for (_, handle) in workers {
            let _ = handle.await;
        }
    }
}

impl Clone for WorkerPool {
    fn clone(&self) -> Self {
        Self {
            workers: Arc::clone(&self.workers),
            max_workers: self.max_workers,
        }
    }
}

pub struct Worker {
    id: Uuid,
    status: WorkerStatus,
}

#[derive(Debug, Clone)]
pub enum WorkerStatus {
    Idle,
    Busy(String), // 任務ID
    Stopped,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            status: WorkerStatus::Idle,
        }
    }
    
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    pub fn status(&self) -> &WorkerStatus {
        &self.status
    }
}
```

##### 2.5 整合到命令系統
```rust
// 修改現有命令檔案，例如 src/commands/match_command.rs
use crate::core::parallel::{TaskScheduler, FileProcessingTask, ProcessingOperation};

pub async fn execute_parallel_match(
    directory: &Path, 
    recursive: bool,
    output: Option<&Path>
) -> Result<()> {
    // 初始化配置和任務調度器
    crate::config::init_config_manager()?;
    let scheduler = TaskScheduler::new()?;
    
    // 發現需要處理的檔案
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(directory, recursive)?;
    
    // 建立批次任務
    let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
    
    for video_file in files.iter().filter(|f| matches!(f.file_type, MediaFileType::Video)) {
        let task = Box::new(FileProcessingTask {
            input_path: video_file.path.clone(),
            output_path: output.unwrap_or(directory).to_path_buf(),
            operation: ProcessingOperation::MatchFiles,
        });
        tasks.push(task);
    }
    
    // 提交批次任務並等待完成
    println!("正在處理 {} 個檔案...", tasks.len());
    let results = scheduler.submit_batch_tasks(tasks).await;
    
    // 處理結果
    let mut success_count = 0;
    let mut failed_count = 0;
    
    for result in results {
        match result {
            TaskResult::Success(msg) => {
                println!("✓ {}", msg);
                success_count += 1;
            }
            TaskResult::Failed(msg) => {
                eprintln!("✗ {}", msg);
                failed_count += 1;
            }
            TaskResult::Cancelled => {
                eprintln!("✗ 任務被取消");
                failed_count += 1;
            }
        }
    }
    
    println!("\n處理完成: {} 成功, {} 失敗", success_count, failed_count);
    
    Ok(())
}
```

### 模組 3: 音訊採樣率動態配置實作

#### 階段 1: 音訊重採樣引擎 (預估工時: 8 小時)

##### 3.1 建立音訊重採樣系統
```rust
// src/services/audio/resampler.rs
use crate::config::load_config;
use crate::Result;
use std::path::Path;

pub struct AudioResampler {
    target_sample_rate: u32,
    quality: ResampleQuality,
}

#[derive(Debug, Clone)]
pub enum ResampleQuality {
    Low,      // 快速但品質較低
    Medium,   // 平衡品質與速度
    High,     // 高品質但較慢
}

pub struct AudioSampleInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
    pub bit_depth: u16,
}

impl AudioResampler {
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        
        Ok(Self {
            target_sample_rate: config.sync.audio_sample_rate,
            quality: ResampleQuality::Medium,
        })
    }
    
    pub fn with_quality(mut self, quality: ResampleQuality) -> Self {
        self.quality = quality;
        self
    }
    
    pub async fn get_audio_info(&self, audio_path: &Path) -> Result<AudioSampleInfo> {
        // 使用 symphonia 分析音訊檔案資訊
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::probe::Hint;
        use std::fs::File;
        
        let file = File::open(audio_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        
        let mut hint = Hint::new();
        if let Some(extension) = audio_path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }
        
        let meta_opts = Default::default();
        let fmt_opts = Default::default();
        
        let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
        let format = probed.format;
        
        // 取得第一個音訊軌道資訊
        let track = format.tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| crate::error::SubXError::audio_processing("找不到音訊軌道".to_string()))?;
        
        let codec_params = &track.codec_params;
        
        Ok(AudioSampleInfo {
            sample_rate: codec_params.sample_rate.unwrap_or(44100),
            channels: codec_params.channels.map(|ch| ch.count() as u16).unwrap_or(2),
            duration_seconds: 0.0, // 需要進一步計算
            bit_depth: codec_params.bits_per_sample.unwrap_or(16) as u16,
        })
    }
    
    pub async fn needs_resampling(&self, audio_path: &Path) -> Result<bool> {
        let info = self.get_audio_info(audio_path).await?;
        Ok(info.sample_rate != self.target_sample_rate)
    }
    
    pub async fn resample_if_needed(&self, audio_path: &Path, output_path: &Path) -> Result<bool> {
        let needs_resample = self.needs_resampling(audio_path).await?;
        
        if needs_resample {
            self.resample_audio(audio_path, output_path).await?;
            Ok(true)
        } else {
            // 如果不需要重採樣，直接複製檔案或回傳原始路徑
            if audio_path != output_path {
                std::fs::copy(audio_path, output_path)?;
            }
            Ok(false)
        }
    }
    
    async fn resample_audio(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // 實作音訊重採樣邏輯
        // 這裡使用 symphonia 進行解碼，然後重採樣後編碼
        
        let input_info = self.get_audio_info(input_path).await?;
        
        println!("重採樣音訊檔案:");
        println!("  輸入: {} Hz → 輸出: {} Hz", input_info.sample_rate, self.target_sample_rate);
        println!("  品質: {:?}", self.quality);
        
        // TODO: 實作實際的重採樣演算法
        // 可以使用 rubato 或其他重採樣庫
        
        // 暫時的模擬實作
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }
    
    pub fn get_recommended_sample_rate(&self, audio_info: &AudioSampleInfo) -> u32 {
        // 根據原始採樣率推薦最佳採樣率
        match audio_info.sample_rate {
            rate if rate <= 22050 => 22050,
            rate if rate <= 44100 => 44100,
            rate if rate <= 48000 => 48000,
            rate if rate <= 96000 => 96000,
            _ => 192000,
        }
    }
    
    pub fn estimate_processing_time(&self, audio_info: &AudioSampleInfo) -> std::time::Duration {
        // 根據音訊長度和品質設定估算處理時間
        let base_time = audio_info.duration_seconds;
        let quality_factor = match self.quality {
            ResampleQuality::Low => 0.1,
            ResampleQuality::Medium => 0.3,
            ResampleQuality::High => 0.8,
        };
        
        std::time::Duration::from_secs_f64(base_time * quality_factor)
    }
}
```

##### 3.2 整合到音訊服務
```rust
// 修改 src/services/audio/mod.rs
pub mod resampler;

pub use resampler::{AudioResampler, AudioSampleInfo, ResampleQuality};

use crate::Result;
use std::path::Path;

/// 音訊分析器
pub struct AudioAnalyzer {
    resampler: AudioResampler,
}

impl AudioAnalyzer {
    pub fn new() -> Result<Self> {
        let resampler = AudioResampler::new()?;
        
        Ok(Self {
            resampler,
        })
    }
    
    pub async fn load_audio_file(&self, audio_path: &Path) -> Result<AudioData> {
        // 首先檢查是否需要重採樣
        let needs_resample = self.resampler.needs_resampling(audio_path).await?;
        
        let processed_path = if needs_resample {
            // 建立臨時檔案進行重採樣
            let temp_path = self.create_temp_audio_path(audio_path)?;
            self.resampler.resample_if_needed(audio_path, &temp_path).await?;
            temp_path
        } else {
            audio_path.to_path_buf()
        };
        
        // 載入處理後的音訊資料
        self.load_processed_audio(&processed_path).await
    }
    
    async fn load_processed_audio(&self, audio_path: &Path) -> Result<AudioData> {
        // 使用現有的音訊載入邏輯
        // 這裡假設已經有基本的音訊載入功能
        
        // 暫時的模擬實作
        Ok(AudioData {
            samples: vec![0.0; 44100], // 1 秒的靜音
            sample_rate: 44100,
            channels: 2,
        })
    }
    
    fn create_temp_audio_path(&self, original_path: &Path) -> Result<std::path::PathBuf> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let temp_name = format!("subx_resampled_{}_{}.wav", 
            timestamp,
            original_path.file_stem().unwrap().to_string_lossy());
        
        let temp_dir = std::env::temp_dir();
        Ok(temp_dir.join(temp_name))
    }
    
    pub async fn get_optimal_sample_rate(&self, audio_path: &Path) -> Result<u32> {
        let info = self.resampler.get_audio_info(audio_path).await?;
        Ok(self.resampler.get_recommended_sample_rate(&info))
    }
}

#[derive(Debug)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug)]
pub struct AudioEnvelope {
    pub energy_levels: Vec<f32>,
    pub sample_rate: u32,
    pub window_size: usize,
}
```

#### 階段 2: 整合到同步引擎 (預估工時: 6 小時)

##### 3.3 更新同步引擎以支援動態採樣率
```rust
// 修改 src/core/sync/engine.rs
use crate::services::audio::{AudioAnalyzer, AudioResampler};

impl SyncEngine {
    pub async fn sync_with_adaptive_sampling(
        &self,
        video_path: &Path,
        subtitle_path: &Path,
    ) -> Result<SyncResult> {
        // 初始化音訊分析器（包含重採樣功能）
        let audio_analyzer = AudioAnalyzer::new()?;
        
        // 取得最佳採樣率建議
        let optimal_sample_rate = audio_analyzer.get_optimal_sample_rate(video_path).await?;
        
        println!("音訊分析:");
        println!("  建議採樣率: {} Hz", optimal_sample_rate);
        
        // 載入並處理音訊（包含自動重採樣）
        let audio_data = audio_analyzer.load_audio_file(video_path).await?;
        
        // 載入字幕
        let subtitles = self.load_subtitles(subtitle_path).await?;
        
        // 執行同步分析
        let sync_result = self.analyze_sync(&audio_data, &subtitles).await?;
        
        Ok(sync_result)
    }
    
    async fn analyze_sync(&self, audio_data: &AudioData, subtitles: &[Subtitle]) -> Result<SyncResult> {
        // 使用優化後的音訊資料進行相關性分析
        let correlation_result = self.calculate_correlation(audio_data, subtitles).await?;
        
        Ok(SyncResult {
            offset_seconds: correlation_result.offset,
            confidence: correlation_result.confidence,
            method_used: SyncMethod::AudioCorrelation,
            correlation_peak: correlation_result.peak_value,
        })
    }
    
    async fn calculate_correlation(&self, audio_data: &AudioData, subtitles: &[Subtitle]) -> Result<CorrelationResult> {
        // 實作音訊與字幕的相關性分析
        // 這裡會使用重採樣後的高品質音訊資料
        
        // 暫時的模擬實作
        Ok(CorrelationResult {
            offset: 0.0,
            confidence: 0.85,
            peak_value: 0.92,
        })
    }
}

struct CorrelationResult {
    offset: f32,
    confidence: f32,
    peak_value: f32,
}
```

### 模組 4: 檔案編碼自動檢測實作

#### 階段 1: 編碼檢測引擎 (預估工時: 10 小時)

##### 4.1 建立編碼檢測系統
```rust
// src/core/formats/encoding.rs
use crate::config::load_config;
use crate::Result;
use std::path::Path;

pub struct EncodingDetector {
    default_encoding: String,
    supported_encodings: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct EncodingInfo {
    pub detected_encoding: String,
    pub confidence: f32,
    pub bom_detected: bool,
    pub sample_text: String,
}

#[derive(Debug, Clone)]
pub enum EncodingError {
    DetectionFailed(String),
    ConversionFailed(String),
    UnsupportedEncoding(String),
}

impl EncodingDetector {
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        
        Ok(Self {
            default_encoding: config.formats.default_encoding,
            supported_encodings: vec![
                "UTF-8", "UTF-16", "UTF-16LE", "UTF-16BE",
                "GB2312", "GBK", "GB18030",
                "BIG5", "BIG5-HKSCS",
                "SHIFT_JIS", "EUC-JP", "ISO-2022-JP",
                "EUC-KR", "ISO-8859-1", "WINDOWS-1252",
            ],
        })
    }
    
    pub async fn detect_encoding(&self, file_path: &Path) -> Result<EncodingInfo> {
        // 讀取檔案的開頭部分進行編碼檢測
        let sample_data = self.read_file_sample(file_path).await?;
        
        // 檢測 BOM (Byte Order Mark)
        let bom_result = self.detect_bom(&sample_data);
        
        if let Some(encoding) = bom_result {
            return Ok(EncodingInfo {
                detected_encoding: encoding.clone(),
                confidence: 1.0,
                bom_detected: true,
                sample_text: self.decode_sample(&sample_data, &encoding)?,
            });
        }
        
        // 如果沒有 BOM，使用啟發式檢測
        let heuristic_result = self.heuristic_detection(&sample_data)?;
        
        Ok(heuristic_result)
    }
    
    async fn read_file_sample(&self, file_path: &Path) -> Result<Vec<u8>> {
        use tokio::fs::File;
        use tokio::io::{AsyncReadExt};
        
        let mut file = File::open(file_path).await?;
        let mut buffer = vec![0u8; 8192]; // 讀取前 8KB 用於檢測
        let bytes_read = file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        
        Ok(buffer)
    }
    
    fn detect_bom(&self, data: &[u8]) -> Option<String> {
        if data.len() >= 3 && data[0..3] == [0xEF, 0xBB, 0xBF] {
            Some("UTF-8".to_string())
        } else if data.len() >= 2 && data[0..2] == [0xFF, 0xFE] {
            Some("UTF-16LE".to_string())
        } else if data.len() >= 2 && data[0..2] == [0xFE, 0xFF] {
            Some("UTF-16BE".to_string())
        } else if data.len() >= 4 && data[0..4] == [0xFF, 0xFE, 0x00, 0x00] {
            Some("UTF-32LE".to_string())
        } else if data.len() >= 4 && data[0..4] == [0x00, 0x00, 0xFE, 0xFF] {
            Some("UTF-32BE".to_string())
        } else {
            None
        }
    }
    
    fn heuristic_detection(&self, data: &[u8]) -> Result<EncodingInfo> {
        let mut best_encoding = self.default_encoding.clone();
        let mut best_confidence = 0.0;
        let mut best_text = String::new();
        
        for &encoding in &self.supported_encodings {
            if let Ok(decoded_text) = self.try_decode(data, encoding) {
                let confidence = self.calculate_confidence(&decoded_text, encoding);
                
                if confidence > best_confidence {
                    best_confidence = confidence;
                    best_encoding = encoding.to_string();
                    best_text = decoded_text;
                }
            }
        }
        
        Ok(EncodingInfo {
            detected_encoding: best_encoding,
            confidence: best_confidence,
            bom_detected: false,
            sample_text: best_text,
        })
    }
    
    fn try_decode(&self, data: &[u8], encoding: &str) -> Result<String> {
        use encoding_rs::Encoding;
        
        let encoding_obj = Encoding::for_label(encoding.as_bytes())
            .ok_or_else(|| EncodingError::UnsupportedEncoding(encoding.to_string()))?;
        
        let (decoded, _, had_errors) = encoding_obj.decode(data);
        
        if had_errors {
            Err(EncodingError::ConversionFailed(format!("解碼 {} 時發生錯誤", encoding)))?
        } else {
            Ok(decoded.into_owned())
        }
    }
    
    fn decode_sample(&self, data: &[u8], encoding: &str) -> Result<String> {
        self.try_decode(data, encoding)
    }
    
    fn calculate_confidence(&self, text: &str, encoding: &str) -> f32 {
        let mut confidence = 0.5; // 基礎信心值
        
        // 檢測字符分布
        let char_distribution = self.analyze_character_distribution(text);
        
        // UTF-8 優先權
        if encoding == "UTF-8" && char_distribution.is_valid_utf8 {
            confidence += 0.3;
        }
        
        // 中文編碼檢測
        if (encoding.contains("GB") || encoding.contains("BIG5")) && char_distribution.chinese_ratio > 0.1 {
            confidence += 0.2;
        }
        
        // 日文編碼檢測
        if encoding.contains("JP") && char_distribution.japanese_ratio > 0.1 {
            confidence += 0.2;
        }
        
        // 韓文編碼檢測
        if encoding.contains("KR") && char_distribution.korean_ratio > 0.1 {
            confidence += 0.2;
        }
        
        // 控制字符檢測（降低信心值）
        if char_distribution.control_char_ratio > 0.1 {
            confidence -= 0.3;
        }
        
        confidence.clamp(0.0, 1.0)
    }
    
    fn analyze_character_distribution(&self, text: &str) -> CharacterDistribution {
        let total_chars = text.chars().count();
        if total_chars == 0 {
            return CharacterDistribution::default();
        }
        
        let mut chinese_count = 0;
        let mut japanese_count = 0;
        let mut korean_count = 0;
        let mut control_count = 0;
        
        for ch in text.chars() {
            let code_point = ch as u32;
            
            // 中文字符範圍
            if (0x4E00..=0x9FFF).contains(&code_point) {
                chinese_count += 1;
            }
            
            // 日文字符範圍
            if (0x3040..=0x309F).contains(&code_point) || (0x30A0..=0x30FF).contains(&code_point) {
                japanese_count += 1;
            }
            
            // 韓文字符範圍
            if (0xAC00..=0xD7AF).contains(&code_point) {
                korean_count += 1;
            }
            
            // 控制字符
            if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
                control_count += 1;
            }
        }
        
        CharacterDistribution {
            is_valid_utf8: text.is_ascii() || text.chars().all(|c| c != '\u{FFFD}'),
            chinese_ratio: chinese_count as f32 / total_chars as f32,
            japanese_ratio: japanese_count as f32 / total_chars as f32,
            korean_ratio: korean_count as f32 / total_chars as f32,
            control_char_ratio: control_count as f32 / total_chars as f32,
        }
    }
    
    pub async fn convert_encoding(&self, file_path: &Path, target_encoding: &str) -> Result<String> {
        // 檢測當前編碼
        let encoding_info = self.detect_encoding(file_path).await?;
        
        // 如果已經是目標編碼，直接讀取
        if encoding_info.detected_encoding == target_encoding {
            return Ok(std::fs::read_to_string(file_path)?);
        }
        
        // 讀取完整檔案
        let file_data = std::fs::read(file_path)?;
        
        // 從檢測到的編碼解碼
        let decoded_text = self.try_decode(&file_data, &encoding_info.detected_encoding)?;
        
        // 如果目標編碼是 UTF-8，直接回傳
        if target_encoding == "UTF-8" {
            return Ok(decoded_text);
        }
        
        // 否則需要重新編碼（這裡簡化處理，實際應該實作完整的編碼轉換）
        Ok(decoded_text)
    }
}

#[derive(Debug, Default)]
struct CharacterDistribution {
    is_valid_utf8: bool,
    chinese_ratio: f32,
    japanese_ratio: f32,
    korean_ratio: f32,
    control_char_ratio: f32,
}
```

##### 4.2 整合到格式管理器
```rust
// 修改 src/core/formats/manager.rs
use crate::core::formats::encoding::{EncodingDetector, EncodingInfo};

impl FormatManager {
    pub async fn parse_auto_with_encoding_detection(&self, content: &str, file_path: Option<&Path>) -> Result<Subtitle> {
        // 如果提供了檔案路徑，先進行編碼檢測
        let processed_content = if let Some(path) = file_path {
            let detector = EncodingDetector::new()?;
            detector.convert_encoding(path, "UTF-8").await?
        } else {
            content.to_string()
        };
        
        // 使用現有的自動解析邏輯
        self.parse_auto(&processed_content)
    }
    
    pub async fn detect_file_encoding(&self, file_path: &Path) -> Result<EncodingInfo> {
        let detector = EncodingDetector::new()?;
        detector.detect_encoding(file_path).await
    }
    
    pub async fn load_subtitle_with_encoding_detection(&self, file_path: &Path) -> Result<Subtitle> {
        // 檢測編碼並轉換為 UTF-8
        let detector = EncodingDetector::new()?;
        let content = detector.convert_encoding(file_path, "UTF-8").await?;
        
        // 解析字幕內容
        self.parse_auto_with_encoding_detection(&content, Some(file_path)).await
    }
}
```

## 技術規範與相依性

### 相依套件需求

#### 新增的 Cargo.toml 相依性
```toml
[dependencies]
# 音訊重採樣
rubato = "0.12"           # 高品質音訊重採樣庫
# encoding_rs 已經包含在現有相依中

# 平行處理
uuid = { version = "1.0", features = ["v4"] }
# tokio 已經包含在現有相依中

# 對話檢測 (symphonia 已包含)
```

### 配置項目對應

#### 統一配置系統整合
所有新功能都必須使用統一配置系統：

```rust
// 正確的配置載入方式
use crate::config::load_config;

let config = load_config()?;
let threshold = config.sync.dialogue_detection_threshold;
let max_jobs = config.general.max_concurrent_jobs;
let sample_rate = config.sync.audio_sample_rate;
let default_encoding = config.formats.default_encoding;
```

### 錯誤處理標準

#### 統一錯誤類型
```rust
// 在 src/error.rs 中新增相關錯誤類型
impl SubXError {
    pub fn dialogue_detection(msg: String) -> Self {
        Self::AudioProcessing(msg)
    }
    
    pub fn parallel_processing(msg: String) -> Self {
        Self::CommandExecution(msg)
    }
    
    pub fn resampling_failed(msg: String) -> Self {
        Self::AudioProcessing(msg)
    }
    
    pub fn encoding_detection(msg: String) -> Self {
        Self::FormatProcessing(msg)
    }
}
```

## 測試策略

#### 單元測試要求
每個新模組都必須包含全面的單元測試：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_dialogue_detection() {
        // 測試對話檢測功能
    }
    
    #[tokio::test]
    async fn test_parallel_task_execution() {
        // 測試平行任務執行
    }
    
    #[tokio::test]
    async fn test_audio_resampling() {
        // 測試音訊重採樣
    }
    
    #[tokio::test]
    async fn test_encoding_detection() {
        // 測試編碼檢測
    }
}
```

#### 整合測試計劃
```rust
// tests/missing_features_integration_tests.rs
#[tokio::test]
async fn test_complete_workflow_with_new_features() {
    // 測試包含所有新功能的完整工作流程
}
```

## 實作優先級與時程

### 第一階段: 基礎功能 (預估 2-3 天)
**優先級**: 高
1. **對話檢測功能** - 為同步引擎提供更精確的時間點
2. **編碼檢測功能** - 解決字符編碼問題

### 第二階段: 效能最佳化 (預估 2-3 天)
**優先級**: 中
3. **音訊重採樣功能** - 提升音訊處理品質
4. **平行處理系統** - 提升批次處理效能

### 實作檢查清單

#### 開發前準備
- [ ] 確認統一配置系統正常運作
- [ ] 檢查現有核心模組的 API 介面
- [ ] 準備測試資料檔案（不同編碼、不同採樣率等）

#### 對話檢測模組
- [ ] 實作 `src/core/sync/dialogue/` 模組結構
- [ ] 整合到現有 `SyncEngine`
- [ ] 編寫單元測試和整合測試
- [ ] 驗證配置載入 `load_config()` 正常運作

#### 平行處理模組
- [ ] 實作 `src/core/parallel/` 模組結構
- [ ] 整合到現有命令系統
- [ ] 實作任務狀態監控
- [ ] 測試資源限制和錯誤處理

#### 音訊重採樣模組
- [ ] 實作 `src/services/audio/resampler.rs`
- [ ] 整合到現有音訊處理服務
- [ ] 支援多種音訊格式
- [ ] 效能基準測試

#### 編碼檢測模組
- [ ] 實作 `src/core/formats/encoding.rs`
- [ ] 整合到現有格式管理器
- [ ] 支援主要字符編碼
- [ ] 編碼轉換錯誤處理

## 品質保證

### 程式碼品質標準
```bash
# 開發過程中必須執行的檢查
cargo fmt
cargo clippy -- -D warnings
cargo test

# 測試覆蓋率檢查
cargo llvm-cov --all-features --workspace --html
```

### 效能基準測試
```rust
// 在 benches/ 目錄下建立效能測試
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_dialogue_detection(c: &mut Criterion) {
    c.bench_function("dialogue_detection", |b| {
        b.iter(|| {
            // 對話檢測效能測試
        })
    });
}

criterion_group!(benches, benchmark_dialogue_detection);
criterion_main!(benches);
```

### 檔案組織結構更新

實作完成後，檔案結構將如下：

```
src/
├── core/
│   ├── formats/
│   │   ├── encoding.rs          # 新增: 編碼檢測
│   │   └── ...
│   ├── parallel/                # 新增: 平行處理系統
│   │   ├── mod.rs
│   │   ├── scheduler.rs
│   │   ├── worker.rs
│   │   └── task.rs
│   ├── sync/
│   │   ├── dialogue/            # 新增: 對話檢測
│   │   │   ├── mod.rs
│   │   │   ├── detector.rs
│   │   │   ├── analyzer.rs
│   │   │   └── segment.rs
│   │   └── ...
├── services/
│   ├── audio/
│   │   ├── resampler.rs         # 新增: 音訊重採樣
│   │   └── ...
```

## 成功標準

### 功能驗證標準
1. **對話檢測**: 能夠準確識別音訊中的語音片段，準確率 > 85%
2. **平行處理**: 支援配置的最大並行數，資源使用率最佳化
3. **音訊重採樣**: 支援常見採樣率轉換，音質保持良好
4. **編碼檢測**: 正確識別主要字符編碼，準確率 > 90%

### 整合驗證標準
- 所有功能都能透過統一配置系統進行配置
- 與現有命令系統完全整合
- 錯誤處理遵循現有標準
- 測試覆蓋率 > 80%

### 效能標準
- 對話檢測處理時間 < 音訊長度的 30%
- 平行處理能有效利用系統資源
- 音訊重採樣不超過原始處理時間的 2 倍
- 編碼檢測響應時間 < 100ms（對於小於 1MB 的檔案）

## 後續維護

### 監控和日誌
- 新增適當的日誌記錄
- 效能指標收集
- 錯誤統計和分析

### 文件更新
- 更新 README.md 中的功能說明
- 新增配置選項文件
- 提供使用範例

### 向後相容性
- 確保現有功能不受影響
- 新功能作為可選增強功能
- 保持 API 穩定性
