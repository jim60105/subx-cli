# Product Backlog #14: 實作缺失功能模組

## 領域範圍
未實作功能開發、配置項目對應功能實作、系統功能完整性提升

## 背景描述

根據配置檔案使用情況分析，發現部分配置項目對應的功能尚未實作，這些功能雖然有配置支援，但缺乏實際的業務邏輯實作。為了讓配置系統完整發揮作用，需要實作這些缺失的功能模組。

## 缺失功能清單

### 1. 對話檢測功能 (Dialogue Detection)
**相關配置**: `sync.dialogue_detection_threshold`, `sync.min_dialogue_duration_ms`

#### 功能描述
- 自動檢測音訊中的對話片段
- 識別語音活動和靜默期間
- 為字幕同步提供更精確的時間點

#### 技術需求
- 音訊能量分析
- 語音活動檢測 (VAD)
- 對話片段分割
- 與現有同步引擎整合

### 2. 平行處理系統 (Parallel Processing)
**相關配置**: `general.max_concurrent_jobs`

#### 功能描述
- 支援多檔案並行處理
- 資源使用量控制
- 任務佇列管理
- 進度追蹤和錯誤處理

#### 技術需求
- 任務調度器
- 工作者執行緒池
- 任務狀態管理
- 資源限制控制

### 3. 音訊採樣率動態配置 (Dynamic Audio Sample Rate)
**相關配置**: `sync.audio_sample_rate`

#### 功能描述
- 支援不同採樣率的音訊檔案
- 自動音訊重採樣
- 採樣率最佳化建議
- 品質與效能平衡

#### 技術需求
- 音訊重採樣演算法
- 採樣率檢測和轉換
- 音訊品質評估
- 效能最佳化

### 4. 檔案編碼自動檢測 (Automatic Encoding Detection)
**相關配置**: `formats.default_encoding`

#### 功能描述
- 自動檢測字幕檔案編碼
- 支援多種字符編碼格式
- 編碼轉換和修復
- 編碼衝突處理

#### 技術需求
- 編碼檢測演算法
- 字符編碼轉換
- 編碼錯誤修復
- Unicode 正規化

## 詳細實作計劃

### 模組 1: 對話檢測功能實作

#### 階段 1: 音訊分析基礎 (預估工時: 12 小時)

##### 1.1 建立對話檢測引擎
```rust
// src/core/dialogue/mod.rs
pub mod detector;
pub mod analyzer;
pub mod segment;

pub use detector::DialogueDetector;
pub use analyzer::{AudioAnalyzer, EnergyAnalyzer};
pub use segment::{DialogueSegment, SilenceSegment};
```

##### 1.2 實作音訊能量分析
```rust
// src/core/dialogue/analyzer.rs
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
// src/core/dialogue/segment.rs
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
// src/core/dialogue/detector.rs
use crate::config::SyncConfig;
use crate::core::dialogue::{EnergyAnalyzer, DialogueSegment};

pub struct DialogueDetector {
    energy_analyzer: EnergyAnalyzer,
    config: SyncConfig,
}

impl DialogueDetector {
    pub fn new(config: SyncConfig) -> Self {
        let energy_analyzer = EnergyAnalyzer::new(
            config.dialogue_detection_threshold,
            config.min_dialogue_duration_ms,
        );
        
        Self {
            energy_analyzer,
            config,
        }
    }
    
    pub async fn detect_dialogue(&self, audio_path: &Path) -> Result<Vec<DialogueSegment>, DialogueError> {
        // 載入音訊檔案
        let audio_data = self.load_audio(audio_path).await?;
        
        // 執行對話檢測
        let segments = self.energy_analyzer.analyze(&audio_data.samples, audio_data.sample_rate);
        
        // 後處理和最佳化
        let optimized_segments = self.optimize_segments(segments);
        
        Ok(optimized_segments)
    }
    
    async fn load_audio(&self, audio_path: &Path) -> Result<AudioData, DialogueError> {
        // 使用現有的 AudioAnalyzer 或實作新的音訊載入器
        todo!("實作音訊載入邏輯")
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

##### 2.2 整合到同步引擎
```rust
// 修改 src/core/sync/engine.rs
use crate::core::dialogue::DialogueDetector;

impl SyncEngine {
    pub async fn sync_with_dialogue_detection(
        &self,
        subtitle_path: &Path,
        audio_path: &Path,
    ) -> Result<Vec<SyncResult>, SyncError> {
        // 檢測對話片段
        let dialogue_detector = DialogueDetector::new(self.config.clone());
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
    ) -> Result<Vec<SyncResult>, SyncError> {
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
    pub fn new(max_concurrent_jobs: usize) -> Self {
        let worker_pool = WorkerPool::new(max_concurrent_jobs);
        let semaphore = Arc::new(Semaphore::new(max_concurrent_jobs));
        
        Self {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            worker_pool,
            semaphore,
            max_concurrent: max_concurrent_jobs,
        }
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
```

##### 2.3 定義任務介面
```rust
// src/core/parallel/task.rs
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Task: Debug + Send + Sync {
    async fn execute(&self) -> TaskResult;
    fn task_type(&self) -> TaskType;
    fn estimated_duration(&self) -> std::time::Duration;
}

#[derive(Debug, Clone)]
pub enum TaskType {
    Match,
    Sync,
    Convert,
    Cache,
}

#[derive(Debug)]
pub enum TaskResult {
    Success(TaskOutput),
    Failed(TaskError),
    Cancelled,
}

#[derive(Debug)]
pub enum TaskOutput {
    MatchResult(MatchResult),
    SyncResult(SyncResult),
    ConvertResult(ConvertResult),
    CacheResult(CacheResult),
}

#[derive(Debug)]
pub struct TaskError {
    pub task_type: TaskType,
    pub error_message: String,
    pub recoverable: bool,
}

// 具體任務實作
#[derive(Debug)]
pub struct MatchTask {
    pub subtitle_path: PathBuf,
    pub video_path: PathBuf,
    pub config: AIConfig,
}

#[async_trait]
impl Task for MatchTask {
    async fn execute(&self) -> TaskResult {
        // 執行字幕匹配邏輯
        match self.perform_match().await {
            Ok(result) => TaskResult::Success(TaskOutput::MatchResult(result)),
            Err(e) => TaskResult::Failed(TaskError {
                task_type: TaskType::Match,
                error_message: e.to_string(),
                recoverable: true,
            }),
        }
    }
    
    fn task_type(&self) -> TaskType {
        TaskType::Match
    }
    
    fn estimated_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30) // 預估 30 秒
    }
}

impl MatchTask {
    async fn perform_match(&self) -> Result<MatchResult, MatchError> {
        // 實際的匹配邏輯
        todo!("實作匹配邏輯")
    }
}
```

#### 階段 2: 工作者池實作 (預估工時: 6 小時)

##### 2.4 實作工作者池
```rust
// src/core/parallel/pool.rs
use tokio::task::JoinHandle;
use std::sync::Arc;

pub struct WorkerPool {
    workers: Vec<Worker>,
    task_sender: mpsc::UnboundedSender<TaskMessage>,
}

pub struct Worker {
    id: usize,
    handle: JoinHandle<()>,
}

enum TaskMessage {
    Execute(Box<dyn Task + Send + Sync>, oneshot::Sender<TaskResult>),
    Shutdown,
}

impl WorkerPool {
    pub fn new(size: usize) -> Self {
        let (task_sender, mut task_receiver) = mpsc::unbounded_channel();
        let mut workers = Vec::new();
        
        for id in 0..size {
            let mut receiver = task_receiver.clone();
            let handle = tokio::spawn(async move {
                while let Some(message) = receiver.recv().await {
                    match message {
                        TaskMessage::Execute(task, result_sender) => {
                            let result = task.execute().await;
                            let _ = result_sender.send(result);
                        }
                        TaskMessage::Shutdown => break,
                    }
                }
            });
            
            workers.push(Worker { id, handle });
        }
        
        Self {
            workers,
            task_sender,
        }
    }
    
    pub async fn execute(&self, task: Box<dyn Task + Send + Sync>) -> TaskResult {
        let (tx, rx) = oneshot::channel();
        
        self.task_sender
            .send(TaskMessage::Execute(task, tx))
            .map_err(|_| TaskError {
                task_type: TaskType::Match, // 預設類型
                error_message: "工作者池已關閉".to_string(),
                recoverable: false,
            })?;
        
        rx.await.unwrap_or(TaskResult::Failed(TaskError {
            task_type: TaskType::Match,
            error_message: "任務執行失敗".to_string(),
            recoverable: true,
        }))
    }
    
    pub async fn shutdown(self) {
        for _ in &self.workers {
            let _ = self.task_sender.send(TaskMessage::Shutdown);
        }
        
        for worker in self.workers {
            let _ = worker.handle.await;
        }
    }
}
```

#### 階段 3: 命令整合 (預估工時: 4 小時)

##### 2.5 整合到命令系統
```rust
// 修改 src/commands/match_command.rs
use crate::core::parallel::{TaskScheduler, MatchTask};

pub async fn handle_match_command(args: &MatchArgs) -> crate::error::Result<()> {
    let config = Config::load(&args.config_path)?;
    
    // 檢查是否需要平行處理
    if args.subtitle_paths.len() > 1 && config.general.max_concurrent_jobs > 1 {
        handle_parallel_match(args, &config).await
    } else {
        handle_single_match(args, &config).await
    }
}

async fn handle_parallel_match(args: &MatchArgs, config: &Config) -> crate::error::Result<()> {
    let scheduler = TaskScheduler::new(config.general.max_concurrent_jobs);
    let mut tasks = Vec::new();
    
    // 建立平行任務
    for subtitle_path in &args.subtitle_paths {
        let task = MatchTask {
            subtitle_path: subtitle_path.clone(),
            video_path: args.video_path.clone(),
            config: config.ai.clone(),
        };
        tasks.push(Box::new(task) as Box<dyn Task + Send + Sync>);
    }
    
    // 執行平行任務
    println!("開始平行處理 {} 個字幕檔案...", tasks.len());
    let results = scheduler.submit_batch_tasks(tasks).await;
    
    // 處理結果
    for (i, result) in results.iter().enumerate() {
        match result {
            TaskResult::Success(output) => {
                println!("✅ 檔案 {} 處理成功", args.subtitle_paths[i].display());
            }
            TaskResult::Failed(error) => {
                eprintln!("❌ 檔案 {} 處理失敗: {}", 
                    args.subtitle_paths[i].display(), error.error_message);
            }
            TaskResult::Cancelled => {
                println!("⚠️ 檔案 {} 處理被取消", args.subtitle_paths[i].display());
            }
        }
    }
    
    Ok(())
}
```

### 模組 3: 音訊採樣率動態配置

#### 階段 1: 音訊重採樣實作 (預估工時: 8 小時)

##### 3.1 建立音訊重採樣器
```rust
// src/core/audio/resampler.rs
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

pub struct AudioResampler {
    target_sample_rate: u32,
    resampler: Option<SincFixedIn>,
}

impl AudioResampler {
    pub fn new(target_sample_rate: u32) -> Self {
        Self {
            target_sample_rate,
            resampler: None,
        }
    }
    
    pub fn resample(&mut self, input: &[f32], input_sample_rate: u32) -> Result<Vec<f32>, ResampleError> {
        if input_sample_rate == self.target_sample_rate {
            return Ok(input.to_vec());
        }
        
        // 初始化重採樣器（如果需要）
        if self.resampler.is_none() || self.needs_resampler_update(input_sample_rate) {
            self.resampler = Some(self.create_resampler(input_sample_rate)?);
        }
        
        let resampler = self.resampler.as_mut().unwrap();
        
        // 執行重採樣
        let mut output = vec![Vec::new(); 1]; // 單聲道
        let mut input_buffer = vec![input.to_vec()];
        
        resampler.process(&input_buffer, &mut output)
            .map_err(|e| ResampleError::ProcessingError(e.to_string()))?;
        
        Ok(output.into_iter().next().unwrap())
    }
    
    fn create_resampler(&self, input_sample_rate: u32) -> Result<SincFixedIn, ResampleError> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        
        let resample_ratio = self.target_sample_rate as f64 / input_sample_rate as f64;
        
        SincFixedIn::new(
            resample_ratio,
            2.0, // 最大相對偏差
            params,
            1024, // 塊大小
            1, // 聲道數
        ).map_err(|e| ResampleError::InitializationError(e.to_string()))
    }
    
    fn needs_resampler_update(&self, input_sample_rate: u32) -> bool {
        // 檢查是否需要更新重採樣器
        true // 簡化實作，實際應該檢查參數變化
    }
}

#[derive(Debug)]
pub enum ResampleError {
    InitializationError(String),
    ProcessingError(String),
    UnsupportedSampleRate(u32),
}
```

##### 3.2 更新音訊分析器
```rust
// 修改 src/services/audio/analyzer.rs
use crate::core::audio::AudioResampler;

impl AudioAnalyzer {
    pub fn new(config: SyncConfig) -> Self {
        let resampler = AudioResampler::new(config.audio_sample_rate);
        
        Self {
            sample_rate: config.audio_sample_rate,
            resampler: Some(resampler),
            // ...其他欄位
        }
    }
    
    pub async fn analyze_with_resampling(&mut self, audio_path: &Path) -> Result<AudioFeatures, AudioError> {
        let audio_data = self.load_audio_file(audio_path).await?;
        
        // 重採樣到目標採樣率
        let resampled_data = if audio_data.sample_rate != self.sample_rate {
            self.resampler.as_mut().unwrap()
                .resample(&audio_data.samples, audio_data.sample_rate)?
        } else {
            audio_data.samples
        };
        
        // 使用重採樣後的資料進行分析
        self.analyze_samples(&resampled_data, self.sample_rate)
    }
}
```

### 模組 4: 檔案編碼自動檢測

#### 階段 1: 編碼檢測實作 (預估工時: 6 小時)

##### 4.1 建立編碼檢測器
```rust
// src/core/encoding/detector.rs
use encoding_rs::{Encoding, Decoder};
use std::fs;

pub struct EncodingDetector {
    default_encoding: &'static Encoding,
    supported_encodings: Vec<&'static Encoding>,
}

impl EncodingDetector {
    pub fn new(default_encoding: &str) -> Result<Self, EncodingError> {
        let default_encoding = Encoding::for_label(default_encoding.as_bytes())
            .ok_or_else(|| EncodingError::UnsupportedEncoding(default_encoding.to_string()))?;
        
        let supported_encodings = vec![
            encoding_rs::UTF_8,
            encoding_rs::UTF_16LE,
            encoding_rs::UTF_16BE,
            encoding_rs::WINDOWS_1252,
            encoding_rs::ISO_8859_1,
            encoding_rs::GB18030,
            encoding_rs::BIG5,
            encoding_rs::SHIFT_JIS,
        ];
        
        Ok(Self {
            default_encoding,
            supported_encodings,
        })
    }
    
    pub fn detect_file_encoding(&self, file_path: &Path) -> Result<DetectionResult, EncodingError> {
        let bytes = fs::read(file_path)
            .map_err(|e| EncodingError::FileError(file_path.to_path_buf(), e))?;
        
        self.detect_encoding(&bytes)
    }
    
    pub fn detect_encoding(&self, bytes: &[u8]) -> Result<DetectionResult, EncodingError> {
        // 檢查 BOM
        if let Some(encoding) = self.detect_bom(bytes) {
            return Ok(DetectionResult {
                encoding,
                confidence: 1.0,
                method: DetectionMethod::BOM,
            });
        }
        
        // 嘗試各種編碼
        let mut best_result = DetectionResult {
            encoding: self.default_encoding,
            confidence: 0.0,
            method: DetectionMethod::Heuristic,
        };
        
        for &encoding in &self.supported_encodings {
            let confidence = self.test_encoding(bytes, encoding);
            if confidence > best_result.confidence {
                best_result = DetectionResult {
                    encoding,
                    confidence,
                    method: DetectionMethod::Heuristic,
                };
            }
        }
        
        // 如果信心度太低，使用預設編碼
        if best_result.confidence < 0.5 {
            best_result.encoding = self.default_encoding;
            best_result.method = DetectionMethod::Default;
        }
        
        Ok(best_result)
    }
    
    fn detect_bom(&self, bytes: &[u8]) -> Option<&'static Encoding> {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            Some(encoding_rs::UTF_8)
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            Some(encoding_rs::UTF_16LE)
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            Some(encoding_rs::UTF_16BE)
        } else {
            None
        }
    }
    
    fn test_encoding(&self, bytes: &[u8], encoding: &'static Encoding) -> f32 {
        let mut decoder = encoding.new_decoder();
        let mut output = String::new();
        
        let (result, _, had_errors) = decoder.decode_to_string(bytes, &mut output, true);
        
        if had_errors {
            return 0.0;
        }
        
        // 計算文字品質分數
        self.calculate_text_quality(&output)
    }
    
    fn calculate_text_quality(&self, text: &str) -> f32 {
        let total_chars = text.chars().count() as f32;
        if total_chars == 0.0 {
            return 0.0;
        }
        
        let printable_chars = text.chars()
            .filter(|c| c.is_ascii_graphic() || c.is_whitespace() || !c.is_control())
            .count() as f32;
        
        // 基本的品質評估
        let quality_score = printable_chars / total_chars;
        
        // 加分項目：包含常見字幕格式標記
        let subtitle_markers = [
            "-->", "Dialogue:", "Style:", "[Script Info]", "{\\", "}"
        ];
        let has_subtitle_markers = subtitle_markers.iter()
            .any(|marker| text.contains(marker));
        
        if has_subtitle_markers {
            quality_score * 1.2 // 提高 20% 信心度
        } else {
            quality_score
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub encoding: &'static Encoding,
    pub confidence: f32,
    pub method: DetectionMethod,
}

#[derive(Debug, Clone)]
pub enum DetectionMethod {
    BOM,      // 透過 Byte Order Mark 檢測
    Heuristic, // 透過啟發式方法檢測
    Default,   // 使用預設編碼
}

#[derive(Debug)]
pub enum EncodingError {
    UnsupportedEncoding(String),
    FileError(PathBuf, std::io::Error),
    ConversionError(String),
}
```

##### 4.2 整合編碼檢測到檔案載入
```rust
// 修改字幕載入邏輯
use crate::core::encoding::EncodingDetector;

impl SubtitleLoader {
    pub fn new(config: FormatsConfig) -> Self {
        let encoding_detector = EncodingDetector::new(&config.default_encoding)
            .unwrap_or_else(|_| EncodingDetector::new("utf-8").unwrap());
        
        Self {
            encoding_detector,
            // ...其他欄位
        }
    }
    
    pub async fn load_with_encoding_detection(&self, file_path: &Path) -> Result<String, LoadError> {
        // 檢測檔案編碼
        let detection_result = self.encoding_detector.detect_file_encoding(file_path)?;
        
        println!("檢測到編碼: {} (信心度: {:.2})", 
            detection_result.encoding.name(), 
            detection_result.confidence);
        
        // 讀取並轉換檔案
        let bytes = tokio::fs::read(file_path).await?;
        let (decoded_text, _, had_errors) = detection_result.encoding.decode(&bytes);
        
        if had_errors {
            eprintln!("⚠️ 警告: 檔案 {} 包含無法解碼的字符", file_path.display());
        }
        
        Ok(decoded_text.into_owned())
    }
}
```

## 測試計劃

### 單元測試覆蓋

#### 對話檢測測試
- [ ] 音訊能量計算測試
- [ ] 語音活動檢測測試
- [ ] 對話片段分割測試
- [ ] 片段合併和最佳化測試

#### 平行處理測試
- [ ] 任務調度器功能測試
- [ ] 工作者池管理測試
- [ ] 任務執行和結果處理測試
- [ ] 錯誤處理和恢復測試

#### 音訊重採樣測試
- [ ] 採樣率轉換準確性測試
- [ ] 音訊品質保持測試
- [ ] 效能和記憶體使用測試

#### 編碼檢測測試
- [ ] BOM 檢測測試
- [ ] 各種編碼格式檢測測試
- [ ] 編碼轉換準確性測試
- [ ] 錯誤編碼恢復測試

### 整合測試場景

#### 端到端功能測試
- [ ] 完整對話檢測流程測試
- [ ] 平行處理多檔案測試
- [ ] 動態採樣率處理測試
- [ ] 自動編碼檢測和轉換測試

## 驗收標準

### 功能驗收
- [ ] 對話檢測功能準確識別語音片段
- [ ] 平行處理系統正確管理並行任務
- [ ] 音訊重採樣保持音訊品質
- [ ] 編碼檢測正確識別常見編碼格式

### 程式碼品質驗收
- [ ] 所有新功能通過單元測試
- [ ] 程式碼覆蓋率 > 80%
- [ ] 通過 clippy 和 fmt 檢查
- [ ] API 文件完整

## 風險評估

### 技術風險
- **中等風險**: 音訊處理演算法複雜度高
- **緩解措施**: 使用成熟的音訊處理函式庫

### 效能風險
- **中等風險**: 平行處理可能增加記憶體使用
- **緩解措施**: 實作資源限制和監控

### 相容性風險
- **低風險**: 新功能主要為新增，不影響現有功能
- **緩解措施**: 保持向後相容性設計

## 完成後效益

### 功能完整性
- 所有配置項目都有對應的功能實作
- 系統功能更加完整和實用
- 使用者體驗更加一致

### 效能提升
- 平行處理大幅提升多檔案處理速度
- 智慧編碼檢測減少使用者手動設定
- 對話檢測提升同步準確性

### 可維護性改善
- 模組化設計易於後續擴展
- 清楚的介面和抽象層
- 完善的測試覆蓋確保品質
