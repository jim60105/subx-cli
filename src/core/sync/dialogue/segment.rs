/// 代表一段對話或靜默的資料結構
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub is_speech: bool,
    pub confidence: f32,
}

impl DialogueSegment {
    /// 建立新的語音片段
    pub fn new_speech(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: true,
            confidence: 1.0,
        }
    }

    /// 建立新的靜默片段
    pub fn new_silence(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: false,
            confidence: 1.0,
        }
    }

    /// 取得片段持續時間（秒）
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// 判斷是否與其他片段重疊
    pub fn overlaps_with(&self, other: &DialogueSegment) -> bool {
        self.start_time < other.end_time && self.end_time > other.start_time
    }

    /// 與其他同類型片段合併，更新邊界與信心度
    pub fn merge_with(&mut self, other: &DialogueSegment) {
        if self.is_speech == other.is_speech && self.overlaps_with(other) {
            self.start_time = self.start_time.min(other.start_time);
            self.end_time = self.end_time.max(other.end_time);
            self.confidence = (self.confidence + other.confidence) / 2.0;
        }
    }
}

/// 靜默片段資料結構，可用於額外處理或分析
#[derive(Debug, Clone)]
pub struct SilenceSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
}

impl SilenceSegment {
    /// 建立靜默片段
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            duration: end - start,
        }
    }

    /// 檢查此靜默片段是否超過最小長度
    pub fn is_significant(&self, min_duration: f64) -> bool {
        self.duration >= min_duration
    }
}
