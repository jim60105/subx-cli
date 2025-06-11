# Bug #15: AI 匹配結果解析修復計畫

## 問題概述

使用者執行 `subx-cli match . --dry-run` 命令時，AI 正確回傳了匹配結果，但系統顯示 "No matching file pairs found"。

### 問題分析

1. **AI 回應**：AI 正確找到了 5 個匹配對，信心度都在 0.95-0.98 之間
2. **系統行為**：顯示 "No matching file pairs found"
3. **根本原因**：檔案名稱比對邏輯存在問題

### 技術根因

在 `src/core/matcher/engine.rs` 第 273-280 行的檔案比對邏輯：

```rust
if let (Some(video), Some(subtitle)) = (
    videos.iter().find(|v| v.name == ai_match.video_file),
    subtitles.iter().find(|s| s.name == ai_match.subtitle_file),
) {
```

**問題點**：
1. `MediaFile.name` 使用 `file_stem()`，不包含副檔名，在 recursive 模式下無法唯一識別檔案
2. AI 回傳的是完整檔名（包含副檔名），但系統比對時使用的是不完整的檔名
3. 缺乏路徑資訊導致在 recursive 模式下同名檔案無法區分
4. 傳送給 AI 的格式為 `"filename (Path: path, Dir: dir)"`，但比對時沒有考慮路徑資訊

## 解決方案設計

### 唯一識別碼導向的檔案匹配架構

完全重設計 `MediaFile` 結構和匹配邏輯，使用唯一識別碼作為主要比對機制，確保在任何情況下都能正確識別和匹配檔案。

#### 實作步驟

1. **導入唯一識別碼系統**
   - 為每個 `MediaFile` 生成唯一的 ID
   - 修改 AI 提示格式以包含識別碼
   - 更新 AI 回傳規格以使用識別碼

2. **重構 MediaFile 結構**
   - 直接修正所有欄位定義
   - 使用完整檔名（包含副檔名）
   - 加入相對路徑和唯一識別碼

3. **更新匹配邏輯**
   - 優先使用唯一識別碼進行比對
   - Fallback 使用路徑比對
   - 完全移除舊的檔名比對邏輯

## 詳細實作計畫

### 第一階段：MediaFile 結構重構和唯一識別碼系統

#### 1.1 重新定義 MediaFile 結構

**檔案**：`src/core/matcher/discovery.rs`

**目標**：建立以唯一識別碼為核心的檔案管理系統

**新結構定義**：

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct MediaFile {
    /// Unique identifier for this media file (deterministic hash)
    pub id: String,
    /// Full path to the media file
    pub path: PathBuf,
    /// Classification of the file (Video or Subtitle)
    pub file_type: MediaFileType,
    /// File size in bytes
    pub size: u64,
    /// Complete filename with extension (e.g., "movie.mkv")
    pub name: String,
    /// File extension (without the dot)
    pub extension: String,
    /// Relative path from scan root for recursive matching
    pub relative_path: String,
}
```

#### 1.2 更新檔案生成邏輯

**檔案**：`src/core/matcher/discovery.rs`

**修改 `classify_file` 方法**：

```rust
fn classify_file(&self, path: &Path, scan_root: &Path) -> Result<Option<MediaFile>> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let file_type = if self.video_extensions.contains(&extension) {
        MediaFileType::Video
    } else if self.subtitle_extensions.contains(&extension) {
        MediaFileType::Subtitle
    } else {
        return Ok(None);
    };

    let metadata = std::fs::metadata(path)?;
    
    // 完整檔名（包含副檔名）
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();

    // 計算相對路徑
    let relative_path = path
        .strip_prefix(scan_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    // 生成唯一識別碼：基於檔案路徑和大小的確定性 hash
    let id = generate_file_id(&relative_path, metadata.len());

    Ok(Some(MediaFile {
        id,
        path: path.to_path_buf(),
        file_type,
        size: metadata.len(),
        name,
        extension,
        relative_path,
    }))
}

/// 生成檔案的確定性唯一識別碼
/// 
/// 使用高效的 hash 算法確保：
/// 1. 相同檔案（路徑+大小）總是產生相同 ID
/// 2. 不同檔案極大機率產生不同 ID  
/// 3. 計算速度快，不影響掃描效能
fn generate_file_id(relative_path: &str, file_size: u64) -> String {
    let mut hasher = DefaultHasher::new();
    relative_path.hash(&mut hasher);
    file_size.hash(&mut hasher);
    
    // 使用 16 位十六進制確保足夠的唯一性 (2^64 種可能)
    format!("file_{:016x}", hasher.finish())
}
```

#### 1.3 更新 scan_directory 方法

**檔案**：`src/core/matcher/discovery.rs`

```rust
pub fn scan_directory(&self, root_path: &Path, recursive: bool) -> Result<Vec<MediaFile>> {
    let mut files = Vec::new();

    let walker = if recursive {
        WalkDir::new(root_path).into_iter()
    } else {
        WalkDir::new(root_path).max_depth(1).into_iter()
    };

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(media_file) = self.classify_file(path, root_path)? {
                files.push(media_file);
            }
        }
    }

    Ok(files)
}
```

### 第二階段：AI 提示格式和回傳規格更新

#### 2.1 修改 AI 提示建構邏輯

**檔案**：`src/core/matcher/engine.rs`

**目標**：在 AI 提示中包含檔案的唯一識別碼

**修改前**：
```rust
let video_files: Vec<String> = videos
    .iter()
    .map(|v| {
        let rel = v.path.strip_prefix(path).unwrap_or(&v.path).to_string_lossy();
        let dir = v.path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or_default();
        format!("{} (Path: {}, Dir: {})", v.name, rel, dir)
    })
    .collect();
```

**修改後**：
```rust
let video_files: Vec<String> = videos
    .iter()
    .map(|v| {
        format!("ID:{} | Name:{} | Path:{}", v.id, v.name, v.relative_path)
    })
    .collect();

let subtitle_files: Vec<String> = subtitles
    .iter()
    .map(|s| {
        format!("ID:{} | Name:{} | Path:{}", s.id, s.name, s.relative_path)
    })
    .collect();
```

#### 2.2 更新 AI 提示模板

**檔案**：`src/services/ai/prompts.rs`

**修改 `build_analysis_prompt` 方法**：

```rust
pub fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str("Please analyze the matching relationship between the following video and subtitle files. Each file has a unique ID that you must use in your response.\n\n");

    prompt.push_str("Video files:\n");
    for video in &request.video_files {
        prompt.push_str(&format!("- {}\n", video));
    }

    prompt.push_str("\nSubtitle files:\n");
    for subtitle in &request.subtitle_files {
        prompt.push_str(&format!("- {}\n", subtitle));
    }

    if !request.content_samples.is_empty() {
        prompt.push_str("\nSubtitle content preview:\n");
        for sample in &request.content_samples {
            prompt.push_str(&format!("File: {}\n", sample.filename));
            prompt.push_str(&format!("Content: {}\n\n", sample.content_preview));
        }
    }

    prompt.push_str(
        "Please provide matching suggestions based on filename patterns, content similarity, and other factors.\n\
        Response format must be JSON using the file IDs:\n\
        {\n\
          \"matches\": [\n\
            {\n\
              \"video_file_id\": \"file_abc123456789abcd\",\n\
              \"subtitle_file_id\": \"file_def456789abcdef0\",\n\
              \"confidence\": 0.95,\n\
              \"match_factors\": [\"filename_similarity\", \"content_correlation\"]\n\
            }\n\
          ],\n\
          \"confidence\": 0.9,\n\
          \"reasoning\": \"Explanation for the matching decisions\"\n\
        }",
    );

    prompt
}
```

#### 2.3 更新 AI 回傳數據結構

**檔案**：`src/services/ai/mod.rs`

**修改 `FileMatch` 結構**：

```rust
/// Individual file match information using unique file IDs.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FileMatch {
    /// Unique ID of the matched video file
    pub video_file_id: String,
    /// Unique ID of the matched subtitle file  
    pub subtitle_file_id: String,
    /// Confidence score for this specific match (0.0 to 1.0)
    pub confidence: f32,
    /// List of factors that contributed to this match
    pub match_factors: Vec<String>,
}
```

### 第三階段：匹配邏輯重構

#### 3.1 更新檔案比對邏輯

**檔案**：`src/core/matcher/engine.rs`

**目標**：使用唯一識別碼作為主要比對機制，路徑作為 fallback

**新的比對函數**：

```rust
/// 使用 ID 優先的檔案查找策略
fn find_media_file_by_id_or_path<'a>(
    files: &'a [MediaFile],
    file_id: &str,
    fallback_path: Option<&str>,
) -> Option<&'a MediaFile> {
    // 優先使用 ID 查找
    if let Some(file) = files.iter().find(|f| f.id == file_id) {
        return Some(file);
    }
    
    // Fallback 到路徑比對
    if let Some(path) = fallback_path {
        if let Some(file) = files.iter().find(|f| f.relative_path == path) {
            return Some(file);
        }
        
        // 最後嘗試檔名比對
        files.iter().find(|f| f.name == path)
    } else {
        None
    }
}
```

**修改主要匹配邏輯**：

**修改前**：
```rust
if let (Some(video), Some(subtitle)) = (
    videos.iter().find(|v| v.name == ai_match.video_file),
    subtitles.iter().find(|s| s.name == ai_match.subtitle_file),
) {
```

**修改後**：
```rust
for ai_match in match_result.matches {
    if ai_match.confidence >= self.config.confidence_threshold {
        let video_match = find_media_file_by_id_or_path(
            &videos, 
            &ai_match.video_file_id,
            None // AI 應該總是返回正確的 ID，不需要 fallback
        );
        
        let subtitle_match = find_media_file_by_id_or_path(
            &subtitles,
            &ai_match.subtitle_file_id, 
            None
        );
        
        match (video_match, subtitle_match) {
            (Some(video), Some(subtitle)) => {
                let new_name = self.generate_subtitle_name(video, subtitle);
                operations.push(MatchOperation {
                    video_file: (*video).clone(),
                    subtitle_file: (*subtitle).clone(),
                    new_subtitle_name: new_name,
                    confidence: ai_match.confidence,
                    reasoning: ai_match.match_factors,
                });
            }
            (None, Some(_)) => {
                eprintln!("⚠️  找不到 AI 建議的影片檔案 ID: '{}'", ai_match.video_file_id);
                self.log_available_files(&videos, "影片");
            }
            (Some(_), None) => {
                eprintln!("⚠️  找不到 AI 建議的字幕檔案 ID: '{}'", ai_match.subtitle_file_id);
                self.log_available_files(&subtitles, "字幕");
            }
            (None, None) => {
                eprintln!("⚠️  找不到 AI 建議的檔案對:");
                eprintln!("     影片 ID: '{}'", ai_match.video_file_id);
                eprintln!("     字幕 ID: '{}'", ai_match.subtitle_file_id);
            }
        }
    } else {
        eprintln!("ℹ️  AI 匹配信心度過低 ({:.2}): {} <-> {}", 
                 ai_match.confidence, ai_match.video_file_id, ai_match.subtitle_file_id);
    }
}
```

#### 3.2 新增除錯輔助函數

**檔案**：`src/core/matcher/engine.rs`

```rust
impl MatchEngine {
    /// 記錄可用檔案以協助除錯
    fn log_available_files(&self, files: &[MediaFile], file_type: &str) {
        eprintln!("   可用的{}檔案:", file_type);
        for f in files {
            eprintln!("     - ID: {} | 名稱: {} | 路徑: {}", 
                     f.id, f.name, f.relative_path);
        }
    }
    
    /// 在沒有找到任何匹配時提供詳細資訊
    fn log_no_matches_found(&self, match_result: &MatchResult, videos: &[MediaFile], subtitles: &[MediaFile]) {
        eprintln!("\n❌ 沒有找到符合條件的檔案匹配");
        eprintln!("🔍 AI 分析結果:");
        eprintln!("   - 總匹配數: {}", match_result.matches.len());
        eprintln!("   - 信心度閾值: {:.2}", self.config.confidence_threshold);
        eprintln!("   - 符合閾值的匹配: {}", 
                 match_result.matches.iter()
                     .filter(|m| m.confidence >= self.config.confidence_threshold)
                     .count());
        
        eprintln!("\n📂 掃描到的檔案:");
        eprintln!("   影片檔案 ({} 個):", videos.len());
        for v in videos {
            eprintln!("     - ID: {} | {}", v.id, v.relative_path);
        }
        eprintln!("   字幕檔案 ({} 個):", subtitles.len());
        for s in subtitles {
            eprintln!("     - ID: {} | {}", s.id, s.relative_path);
        }
    }
}
```

### 第四階段：依賴項目更新

#### 4.1 更新 Cargo.toml 依賴

**檔案**：`Cargo.toml`

**說明**：使用 hash 方案無需新增依賴項，因為使用 Rust 標準庫的 `DefaultHasher`。

**效能考量**：
```toml
# 無需新增依賴
# [dependencies]
# uuid = "1.0"  # 不使用 UUID 方案

# Hash 方案優勢：
# 1. 零額外依賴
# 2. 確定性 ID 生成
# 3. 高效能運算（微秒級）
# 4. 緩存友好
```

#### 4.2 更新其他使用 MediaFile 的模組

**檔案**：`src/commands/sync_command.rs`

**修改檔案查找邏輯**：
```rust
// 修改前
if let Some(s) = subs.iter().find(|s| s.name == video.name) {

// 修改後  
if let Some(s) = subs.iter().find(|s| {
    // 移除副檔名後比較基礎名稱
    let video_base = video.name.strip_suffix(&format!(".{}", video.extension))
        .unwrap_or(&video.name);
    let sub_base = s.name.strip_suffix(&format!(".{}", s.extension))
        .unwrap_or(&s.name);
    video_base == sub_base
}) {
```

**檔案**：`src/core/formats/converter.rs`

**更新檔案掃描邏輯**：
```rust
// 確保 converter 模組正確處理新的 MediaFile 結構
let paths = media_files
    .into_iter()
    .filter(|f| {
        matches!(
            f.file_type,
            crate::core::matcher::discovery::MediaFileType::Subtitle
        )
    })
    .map(|f| f.path)  // 使用 path 欄位，行為不變
    .collect();
```

### 第五階段：測試和驗證

#### 5.1 單元測試

#### 5.4 效能基準測試

**檔案**：`benches/file_id_generation_bench.rs`

**新增基準測試**：
```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::hint::black_box;

/// 生成檔案的確定性唯一識別碼
fn generate_file_id(relative_path: &str, file_size: u64) -> String {
    let mut hasher = DefaultHasher::new();
    relative_path.hash(&mut hasher);
    file_size.hash(&mut hasher);
    
    format!("file_{:016x}", hasher.finish())
}

fn bench_file_id_generation(c: &mut Criterion) {
    c.bench_function("file_id_generation_single", |b| {
        b.iter(|| {
            generate_file_id(
                black_box("test/directory/very_long_filename_with_unicode_字幕檔案.mkv"),
                black_box(1024 * 1024 * 1024), // 1GB
            )
        })
    });
    
    c.bench_function("file_id_generation_batch_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                generate_file_id(
                    black_box(&format!("season{}/episode{:03}.mkv", i / 10 + 1, i)),
                    black_box(1000000 + i as u64),
                );
            }
        })
    });
    
    c.bench_function("file_id_generation_batch_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                generate_file_id(
                    black_box(&format!("movies/year{}/movie_{:04}.mkv", 2020 + i / 100, i)),
                    black_box(1000000 + i as u64),
                );
            }
        })
    });
    
    // 測試不同長度的檔案路徑
    c.bench_function("file_id_generation_long_path", |b| {
        let long_path = "very/long/directory/structure/with/many/nested/folders/and/unicode/characters/影片/字幕/季節一/第一集/最終檔案名稱.mkv";
        b.iter(|| {
            generate_file_id(black_box(long_path), black_box(5000000000)) // 5GB
        })
    });
}

fn bench_id_collision_resistance(c: &mut Criterion) {
    use std::collections::HashSet;
    
    c.bench_function("collision_test_10000_files", |b| {
        b.iter(|| {
            let mut ids = HashSet::new();
            let mut collisions = 0;
            
            for i in 0..10000 {
                let id = generate_file_id(
                    black_box(&format!("test_dir_{}/file_{:06}.mkv", i / 1000, i)),
                    black_box(1000000 + (i * 137) as u64), // 使用質數避免規律性
                );
                
                if !ids.insert(id) {
                    collisions += 1;
                }
            }
            
            black_box(collisions) // 預期為 0
        })
    });
}

criterion_group!(benches, bench_file_id_generation, bench_id_collision_resistance);
criterion_main!(benches);
```

**檔案**：`Cargo.toml`

**新增 criterion 依賴**：
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "file_id_generation_bench"
harness = false
```

**效能目標**：
- 單次 ID 生成：< 1 微秒
- 100 個檔案批次處理：< 100 微秒
- 1000 個檔案批次處理：< 1 毫秒
- 10000 個檔案零衝突率

**執行基準測試**：
```bash
cargo bench
```

**檔案**：`src/core/matcher/discovery.rs`（測試區塊）

**保留簡化的單元測試**：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_media_file_structure_with_unique_id() {
        let temp = TempDir::new().unwrap();
        let video_path = temp.path().join("[Test][01].mkv");
        fs::write(&video_path, b"dummy content").unwrap();
        
        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), false).unwrap();
        
        let video_file = files.iter()
            .find(|f| matches!(f.file_type, MediaFileType::Video))
            .unwrap();
        
        // 測試唯一識別碼
        assert!(!video_file.id.is_empty());
        assert!(video_file.id.starts_with("file_"));
        assert_eq!(video_file.id.len(), 21); // "file_" + 16位十六進制
        
        // 測試完整檔名
        assert_eq!(video_file.name, "[Test][01].mkv");
        
        // 測試副檔名
        assert_eq!(video_file.extension, "mkv");
        
        // 測試相對路徑
        assert_eq!(video_file.relative_path, "[Test][01].mkv");
    }
    
    #[test]
    fn test_deterministic_id_generation() {
        // 測試相同檔案生成相同 ID
        let id1 = generate_file_id("test/file.mkv", 1000);
        let id2 = generate_file_id("test/file.mkv", 1000);
        assert_eq!(id1, id2);
        
        // 測試不同檔案生成不同 ID
        let id3 = generate_file_id("test/file2.mkv", 1000);
        assert_ne!(id1, id3);
        
        let id4 = generate_file_id("test/file.mkv", 2000);
        assert_ne!(id1, id4);
        
        // 測試 ID 格式
        assert!(id1.starts_with("file_"));
        assert_eq!(id1.len(), 21);
    }
    
    #[test]
    fn test_recursive_mode_with_unique_ids() {
        let temp = TempDir::new().unwrap();
        
        // 建立子目錄結構
        let sub_dir = temp.path().join("season1");
        fs::create_dir_all(&sub_dir).unwrap();
        
        let video1 = temp.path().join("movie.mkv");
        let video2 = sub_dir.join("episode1.mkv");
        fs::write(&video1, b"content1").unwrap();
        fs::write(&video2, b"content2").unwrap();
        
        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), true).unwrap();
        
        // 測試不同檔案有不同 ID
        let root_video = files.iter()
            .find(|f| f.name == "movie.mkv")
            .unwrap();
        let sub_video = files.iter()
            .find(|f| f.name == "episode1.mkv")
            .unwrap();
        
        assert_ne!(root_video.id, sub_video.id);
        assert_eq!(root_video.relative_path, "movie.mkv");
        assert_eq!(sub_video.relative_path, "season1/episode1.mkv");
    }
    
    #[test]
    fn test_hash_generation_basic() {
        // 基本功能測試，不關注效能
        let id = generate_file_id("test/file.mkv", 1000);
        assert!(id.starts_with("file_"));
        assert_eq!(id.len(), 21);
    }
}
```

**檔案**：`src/core/matcher/engine.rs`（測試區塊）

**新增測試用例**：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::matcher::discovery::{MediaFile, MediaFileType};
    use std::path::PathBuf;
    
    fn create_test_media_file(id: &str, name: &str, relative_path: &str, file_type: MediaFileType) -> MediaFile {
        MediaFile {
            id: id.to_string(),
            path: PathBuf::from(relative_path),
            file_type,
            size: 1000,
            name: name.to_string(),
            extension: "mkv".to_string(),
            relative_path: relative_path.to_string(),
        }
    }
    
    #[test]
    fn test_find_media_file_by_id_or_path() {
        let files = vec![
            create_test_media_file(
                "file_abc123456789abcd",
                "movie.mkv",
                "movie.mkv",
                MediaFileType::Video
            ),
            create_test_media_file(
                "file_def456789abcdef0",
                "episode1.mkv",
                "season1/episode1.mkv",
                MediaFileType::Video
            )
        ];
        
        // 測試 ID 查找
        assert!(find_media_file_by_id_or_path(&files, "file_abc123456789abcd", None).is_some());
        assert!(find_media_file_by_id_or_path(&files, "file_def456789abcdef0", None).is_some());
        
        // 測試路徑 fallback
        assert!(find_media_file_by_id_or_path(&files, "wrong_id", Some("movie.mkv")).is_some());
        assert!(find_media_file_by_id_or_path(&files, "wrong_id", Some("season1/episode1.mkv")).is_some());
        
        // 測試找不到的情況
        assert!(find_media_file_by_id_or_path(&files, "wrong_id", None).is_none());
        assert!(find_media_file_by_id_or_path(&files, "wrong_id", Some("nonexistent.mkv")).is_none());
    }
    
    #[test]
    fn test_id_prioritization_over_path() {
        let files = vec![
            create_test_media_file(
                "file_correct_id",
                "movie.mkv",
                "movie.mkv",
                MediaFileType::Video
            ),
            create_test_media_file(
                "file_another_id",
                "another.mkv",
                "different/path.mkv",
                MediaFileType::Video
            )
        ];
        
        // 即使 fallback path 匹配其他檔案，ID 匹配應該優先
        let result = find_media_file_by_id_or_path(&files, "file_correct_id", Some("different/path.mkv"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "file_correct_id");
    }
}
```

**檔案**：`src/services/ai/prompts.rs`（測試區塊）

**新增測試用例**：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai::AnalysisRequest;
    
    #[test]
    fn test_ai_prompt_with_file_ids_english() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4".into(), 0.1, 0, 0);
        let request = AnalysisRequest {
            video_files: vec!["ID:file_abc123456789abcd | Name:movie.mkv | Path:movie.mkv".into()],
            subtitle_files: vec!["ID:file_def456789abcdef0 | Name:movie.srt | Path:movie.srt".into()],
            content_samples: vec![],
        };
        
        let prompt = client.build_analysis_prompt(&request);
        
        // 測試包含 ID 和英文提示
        assert!(prompt.contains("ID:file_abc123456789abcd"));
        assert!(prompt.contains("video_file_id"));
        assert!(prompt.contains("subtitle_file_id"));
        assert!(prompt.contains("Please analyze the matching"));
        assert!(prompt.contains("unique ID"));
        assert!(prompt.contains("Response format must be JSON"));
        
        // 確保是英文提示
        assert!(!prompt.contains("請分析"));
        assert!(!prompt.contains("影片檔案"));
        assert!(!prompt.contains("字幕檔案"));
    }
    
    #[test]
    fn test_parse_match_result_with_ids() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4".into(), 0.1, 0, 0);
        let json_resp = r#"{
            "matches": [{
                "video_file_id": "file_abc123456789abcd",
                "subtitle_file_id": "file_def456789abcdef0",
                "confidence": 0.95,
                "match_factors": ["filename_similarity"]
            }],
            "confidence": 0.9,
            "reasoning": "Strong match based on filename patterns"
        }"#;
        
        let result = client.parse_match_result(json_resp).unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].video_file_id, "file_abc123456789abcd");
        assert_eq!(result.matches[0].subtitle_file_id, "file_def456789abcdef0");
        assert_eq!(result.matches[0].confidence, 0.95);
        assert_eq!(result.matches[0].match_factors[0], "filename_similarity");
    }
    
    #[test]
    fn test_ai_prompt_structure_consistency() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4".into(), 0.1, 0, 0);
        let request = AnalysisRequest {
            video_files: vec![
                "ID:file_video1 | Name:video1.mkv | Path:season1/video1.mkv".into(),
                "ID:file_video2 | Name:video2.mkv | Path:season1/video2.mkv".into(),
            ],
            subtitle_files: vec![
                "ID:file_sub1 | Name:sub1.srt | Path:season1/sub1.srt".into(),
                "ID:file_sub2 | Name:sub2.srt | Path:season1/sub2.srt".into(),
            ],
            content_samples: vec![],
        };
        
        let prompt = client.build_analysis_prompt(&request);
        
        // 確保所有檔案 ID 都出現在提示中
        assert!(prompt.contains("ID:file_video1"));
        assert!(prompt.contains("ID:file_video2"));
        assert!(prompt.contains("ID:file_sub1"));
        assert!(prompt.contains("ID:file_sub2"));
        
        // 確保提示結構正確
        assert!(prompt.contains("Video files:"));
        assert!(prompt.contains("Subtitle files:"));
        assert!(prompt.contains("Response format must be JSON"));
    }
}
```

#### 5.2 集成測試

**檔案**：`tests/match_command_integration_tests.rs`

**測試場景**：
```rust
#[tokio::test]
async fn test_match_command_with_file_ids() {
    let temp_dir = tempdir().unwrap();
    let root = temp_dir.path();
    
    // 建立測試檔案
    let season1 = root.join("Season 1");
    fs::create_dir_all(&season1).unwrap();
    
    fs::write(season1.join("[Series][S01E01].mkv"), b"video content").unwrap();
    fs::write(season1.join("[Series][S01E01].srt"), b"subtitle content").unwrap();
    
    // 測試使用 ID 的匹配
    let args = MatchArgs {
        path: root.to_path_buf(),
        recursive: true,
        dry_run: true,
        confidence: 80,
        backup: false,
    };
    
    // 建立模擬 AI 客戶端，使用新的 ID 格式
    let mock_ai = create_mock_ai_with_id_response();
    let config = create_test_config();
    
    let result = execute_with_client(args, Box::new(mock_ai), &config).await;
    assert!(result.is_ok());
}

fn create_mock_ai_with_id_response() -> MockAIClient {
    let mut mock = MockAIClient::new();
    
    mock.expect_analyze_content()
        .returning(|request| {
            // 解析輸入的檔案 ID
            let video_id = extract_id_from_file_info(&request.video_files[0]);
            let subtitle_id = extract_id_from_file_info(&request.subtitle_files[0]);
            
            Ok(MatchResult {
                matches: vec![
                    FileMatch {
                        video_file_id: video_id,
                        subtitle_file_id: subtitle_id,
                        confidence: 0.95,
                        match_factors: vec!["filename_match".to_string()],
                    }
                ],
                confidence: 0.95,
                reasoning: "Strong ID-based match".to_string(),
            })
        });
    
    mock
}

fn extract_id_from_file_info(file_info: &str) -> String {
    // 從 "ID:file_abc123 | Name:... | Path:..." 格式中提取 ID
    file_info.split('|')
        .next()
        .unwrap()
        .trim()
        .strip_prefix("ID:")
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn test_real_world_file_structure() {
    // 模擬使用者提供的實際檔案結構
    let temp_dir = tempdir().unwrap();
    let root = temp_dir.path();
    
    // 建立複雜的檔案結構
    fs::write(root.join("[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].mkv"), 
              b"video content 1").unwrap();
    fs::write(root.join("[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].mkv"), 
              b"video content 2").unwrap();
    fs::write(root.join("[Yozakura-san Chi no Daisakusen][02][BDRIP][1080P][H264_AC3].mkv"), 
              b"video content 3").unwrap();
    fs::write(root.join("[Yozakura-san Chi no Daisakusen][03][BDRIP][1080P][H264_AC3].mkv"), 
              b"video content 4").unwrap();
    
    fs::write(root.join("Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.srt"), 
              b"subtitle content 1").unwrap();
    fs::write(root.join("Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.vtt"), 
              b"subtitle content 2").unwrap();
    fs::write(root.join("夜桜さんちの大作戦 第01話 「桜の指輪」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass"), 
              b"subtitle content 3").unwrap();
    fs::write(root.join("夜桜さんちの大作戦 第02話 「夜桜の命」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass"), 
              b"subtitle content 4").unwrap();
    fs::write(root.join("夜桜さんちの大作戦 第03話 「気持ち」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass"), 
              b"subtitle content 5").unwrap();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
    };
    
    // 建立模擬真實 AI 回應的客戶端
    let mock_ai = create_realistic_ai_response();
    let config = create_test_config();
    
    let result = execute_with_client(args, Box::new(mock_ai), &config).await;
    assert!(result.is_ok());
}

fn create_realistic_ai_response() -> MockAIClient {
    let mut mock = MockAIClient::new();
    
    mock.expect_analyze_content()
        .returning(|request| {
            // 模擬真實的匹配邏輯，返回與使用者相同的 5 個匹配
            let matches = vec![
                FileMatch {
                    video_file_id: extract_id_from_file_info(&request.video_files[0]),
                    subtitle_file_id: extract_id_from_file_info(&request.subtitle_files[2]),
                    confidence: 0.98,
                    match_factors: vec!["episode_number".to_string(), "series_title".to_string()],
                },
                FileMatch {
                    video_file_id: extract_id_from_file_info(&request.video_files[1]),
                    subtitle_file_id: extract_id_from_file_info(&request.subtitle_files[3]),
                    confidence: 0.98,
                    match_factors: vec!["episode_number".to_string(), "series_title".to_string()],
                },
                // ... 更多匹配
            ];
            
            Ok(MatchResult {
                matches,
                confidence: 0.95,
                reasoning: "Strong matches based on filename analysis and episode numbering".to_string(),
            })
        });
    
    mock
}
```

#### 5.3 回歸測試

使用用戶提供的實際檔案結構進行測試：

**測試檔案結構**：
```
test_directory/
├── '[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].mkv'
├── '[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].mkv'
├── '[Yozakura-san Chi no Daisakusen][02][BDRIP][1080P][H264_AC3].mkv'
├── '[Yozakura-san Chi no Daisakusen][03][BDRIP][1080P][H264_AC3].mkv'
├── 'Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.srt'
├── 'Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.vtt'
├── '夜桜さんちの大作戦 第01話 「桜の指輪」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass'
├── '夜桜さんちの大作戰 第02話 「夜桜の命」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass'
└── '夜桜さんちの大作戰 第03話 「気持ち」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass'
```

**預期行為**：
1. 每個檔案分配唯一 ID（例如：`file_a1b2c3d4e5f6789a`）
2. AI 收到包含 ID 的檔案資訊：
   ```
   ID:file_a1b2c3d4e5f6789a | Name:movie.mkv | Path:movie.mkv
   ```
3. AI 返回基於 ID 的匹配結果：
   ```json
   {
     "matches": [{
       "video_file_id": "file_a1b2c3d4e5f6789a",
       "subtitle_file_id": "file_b2c3d4e5f6789ab0",
       "confidence": 0.95,
       "match_factors": ["filename_similarity"]
     }]
   }
   ```
4. 系統使用 ID 成功找到並匹配檔案
5. 輸出 5 個成功的匹配對，而不是 "No matching file pairs found"

**測試用例**：
```rust
#[test]
fn test_user_reported_issue_regression() {
    // 實際重現用戶問題的測試
    let temp_dir = create_user_file_structure();
    
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(temp_dir.path(), false).unwrap();
    
    // 驗證檔案掃描結果
    let videos: Vec<_> = files.iter().filter(|f| matches!(f.file_type, MediaFileType::Video)).collect();
    let subtitles: Vec<_> = files.iter().filter(|f| matches!(f.file_type, MediaFileType::Subtitle)).collect();
    
    assert_eq!(videos.len(), 4);
    assert_eq!(subtitles.len(), 5);
    
    // 驗證每個檔案都有唯一 ID
    let mut ids = std::collections::HashSet::new();
    for file in &files {
        assert!(file.id.starts_with("file_"));
        assert_eq!(file.id.len(), 21); // "file_" + 16位十六進制
        assert!(ids.insert(file.id.clone())); // 確保唯一性
        
        // 驗證完整檔名
        assert!(file.name.contains('.'));
        assert!(!file.name.is_empty());
        
        // 驗證相對路徑
        assert_eq!(file.relative_path, file.name); // 非 recursive 模式
    }
    
    // 模擬 AI 回應格式
    let ai_response = simulate_ai_response_with_ids(&videos, &subtitles);
    
    // 驗證能正確解析和匹配
    for ai_match in ai_response.matches {
        let video = videos.iter().find(|v| v.id == ai_match.video_file_id);
        let subtitle = subtitles.iter().find(|s| s.id == ai_match.subtitle_file_id);
        
        assert!(video.is_some(), "找不到影片檔案 ID: {}", ai_match.video_file_id);
        assert!(subtitle.is_some(), "找不到字幕檔案 ID: {}", ai_match.subtitle_file_id);
    }
}

fn create_user_file_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // 建立用戶提供的檔案結構
    let files = vec![
        ("[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].mkv", b"video1"),
        ("[Yozakura-san Chi no Daisakusen][01][BDRIP][1080P][H264_AC3].mkv", b"video2"),
        ("[Yozakura-san Chi no Daisakusen][02][BDRIP][1080P][H264_AC3].mkv", b"video3"),
        ("[Yozakura-san Chi no Daisakusen][03][BDRIP][1080P][H264_AC3].mkv", b"video4"),
        ("Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.srt", b"sub1"),
        ("Noumin Kanren no Skill S01E01-[1080p][BDRIP][x265.FLAC].cht.vtt", b"sub2"),
        ("夜桜さんちの大作戦 第01話 「桜の指輪」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass", b"sub3"),
        ("夜桜さんちの大作戰 第02話 「夜桜の命」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass", b"sub4"),
        ("夜桜さんちの大作戰 第03話 「気持ち」 (BD 1920x1080 SVT-AV1 ALAC).tc.ass", b"sub5"),
    ];
    
    for (filename, content) in files {
        fs::write(root.join(filename), content).unwrap();
    }
    
    temp_dir
}

fn simulate_ai_response_with_ids(videos: &[&MediaFile], subtitles: &[&MediaFile]) -> MatchResult {
    // 模擬 AI 返回與用戶相同的 5 個匹配
    MatchResult {
        matches: vec![
            FileMatch {
                video_file_id: videos[1].id.clone(), // Yozakura 01
                subtitle_file_id: subtitles[2].id.clone(), // 夜桜 01
                confidence: 0.98,
                match_factors: vec!["episode_number".to_string(), "series_title".to_string()],
            },
            FileMatch {
                video_file_id: videos[2].id.clone(), // Yozakura 02  
                subtitle_file_id: subtitles[3].id.clone(), // 夜桜 02
                confidence: 0.98,
                match_factors: vec!["episode_number".to_string(), "series_title".to_string()],
            },
            FileMatch {
                video_file_id: videos[3].id.clone(), // Yozakura 03
                subtitle_file_id: subtitles[4].id.clone(), // 夜桜 03
                confidence: 0.98,
                match_factors: vec!["episode_number".to_string(), "series_title".to_string()],
            },
            FileMatch {
                video_file_id: videos[0].id.clone(), // Noumin
                subtitle_file_id: subtitles[0].id.clone(), // Noumin SRT
                confidence: 0.95,
                match_factors: vec!["series_title".to_string(), "episode_number".to_string()],
            },
            FileMatch {
                video_file_id: videos[0].id.clone(), // Noumin
                subtitle_file_id: subtitles[1].id.clone(), // Noumin VTT
                confidence: 0.95,
                match_factors: vec!["series_title".to_string(), "episode_number".to_string()],
            },
        ],
        confidence: 0.95,
        reasoning: "Strong matches based on filename patterns and episode numbers".to_string(),
    }
}
```

### 第六階段：文檔更新

#### 6.1 更新技術文檔

**檔案**：`docs/tech-architecture.md`

**新增章節**：
```markdown
## 檔案唯一識別碼系統

### 概述
SubX 使用確定性哈希算法為每個檔案生成唯一識別碼，確保在 AI 匹配過程中能夠精確識別檔案。

### ID 生成算法
- **基礎資料**：檔案的相對路徑和檔案大小
- **算法**：Rust 標準庫的 DefaultHasher
- **格式**：`file_{16位十六進制哈希值}`
- **特性**：確定性（相同檔案總是產生相同 ID）

### 效能特點
- **計算速度**：微秒級 hash 運算，比檔案 I/O 快數千倍
- **記憶體使用**：僅 hash 路徑字串和檔案大小，不讀取檔案內容
- **衝突機率**：使用 64-bit hash，衝突機率極低（約 1/2^64）

### AI 通訊協議
**發送給 AI 的檔案資訊格式**：
```
ID:file_a1b2c3d4e5f6789a | Name:movie.mkv | Path:subdir/movie.mkv
```

**AI 回傳的匹配結果格式**：
```json
{
  "matches": [{
    "video_file_id": "file_a1b2c3d4e5f6789a",
    "subtitle_file_id": "file_b2c3d4e5f6789ab",
    "confidence": 0.95,
    "match_factors": ["filename_similarity"]
  }],
  "confidence": 0.95,
  "reasoning": "匹配說明"
}
```

### 匹配策略
1. **主要策略**：使用唯一 ID 進行精確匹配
2. **備用策略**：ID 匹配失敗時使用路徑匹配
3. **錯誤處理**：提供詳細的診斷資訊，包含可用檔案的 ID 和路徑

### 破壞性變更
- `MediaFile.name` 現在包含完整檔名（含副檔名）
- `FileMatch` 結構改用 `video_file_id` 和 `subtitle_file_id` 欄位
- AI 提示詞格式完全更新為英文，包含檔案 ID 資訊
```

#### 6.2 更新 API 文檔

**檔案**：`docs/api-reference.md`（如果存在）

**新增 MediaFile 結構說明**：
```markdown
## MediaFile 結構

```rust
pub struct MediaFile {
    /// 檔案的唯一識別碼（確定性 hash）
    pub id: String,
    /// 檔案的完整路徑
    pub path: PathBuf,
    /// 檔案類型（影片或字幕）
    pub file_type: MediaFileType,
    /// 檔案大小（位元組）
    pub size: u64,
    /// 完整檔名（含副檔名）
    pub name: String,
    /// 副檔名（不含點號）
    pub extension: String,
    /// 相對於掃描根目錄的路徑
    pub relative_path: String,
}
```

### 重要變更
- `name` 欄位現在包含完整檔名，例如 `"movie.mkv"` 而非 `"movie"`
- 新增 `id` 欄位提供唯一識別
- 新增 `relative_path` 欄位支援 recursive 模式
```

## 風險評估

### 高風險
- **破壞性變更**：`MediaFile.name` 和 `FileMatch` 結構的變更將影響現有代碼
- **AI 回傳格式變更**：需要確保 AI 能正確解析新的英文提示格式並返回期望的 ID 結構
- **Hash 衝突風險**：雖然機率極低，但理論上存在不同檔案產生相同 ID 的可能

### 中風險  
- **效能影響**：ID 生成和比對過程可能略微影響掃描效能
- **測試覆蓋率**：需要確保所有新功能都有充分的測試
- **多語言檔名處理**：需要確保 hash 算法對 Unicode 檔名的穩定性

### 低風險
- **向後兼容性**：由於不考慮向後兼容，可以直接修改而無需額外邏輯
- **依賴管理**：使用標準庫無需新增外部依賴

### 風險緩解措施
1. **Hash 衝突處理**：在發現衝突時提供清晰的錯誤訊息
2. **效能監控**：在測試中加入效能基準測試
3. **詳細測試**：包含各種檔名格式的測試用例
4. **診斷工具**：提供詳細的 debug 資訊協助問題排查

## 成功標準

1. ✅ 每個檔案都有唯一且確定性的識別碼（格式：`file_{16位十六進制}`）
2. ✅ AI 能正確解析包含 ID 的英文檔案資訊
3. ✅ AI 回傳基於 ID 的匹配結果（使用 `video_file_id` 和 `subtitle_file_id` 欄位）
4. ✅ 系統優先使用 ID 進行檔案比對，路徑作為 fallback
5. ✅ 用戶提供的檔案結構能正確匹配（5 個匹配對）
6. ✅ Recursive 和非 recursive 模式都正常工作
7. ✅ 提供清晰的錯誤診斷資訊（包含 ID 和路徑資訊）
8. ✅ 代碼品質檢查全部通過（`cargo fmt` 和 `cargo clippy`）
9. ✅ 測試覆蓋率達到預期水準
10. ✅ 文檔完整且準確，包含破壞性變更說明
11. ✅ Hash 生成效能滿足要求（10000 次操作 < 100ms）
12. ✅ 支援多語言檔名（中文、日文等 Unicode 字符）
13. ✅ 處理複雜的多語言檔名和特殊字符

這個方案確保了在任何情況下（包括同名檔案在不同目錄）都能精確識別和匹配檔案，為 SubX 提供穩固的檔案管理基礎。

## 時程規劃

- **第一階段**：2-3 天（MediaFile 結構重構和唯一識別碼系統）
- **第二階段**：2-3 天（AI 提示格式和回傳規格更新）
- **第三階段**：2-3 天（匹配邏輯重構）
- **第四階段**：1-2 天（依賴項目更新）
- **第五階段**：3-4 天（測試和驗證，包含效能測試）
- **第六階段**：1-2 天（文檔更新）

**總計**：11-17 天

## 實作優先順序

### P0（關鍵路徑）
1. MediaFile 結構重構
2. 唯一識別碼生成系統
3. AI 提示格式更新（英文）
4. 匹配邏輯重構

### P1（重要功能）
1. 錯誤診斷增強
2. 效能優化
3. 完整測試套件

### P2（品質提升）
1. 文檔更新
2. 代碼重構
3. 邊緣案例處理

## 備註

此修復採用全新的檔案識別機制，徹底解決 AI 匹配結果解析問題。主要改進包括：

### 🎯 核心改進
1. **唯一識別碼系統**：每個檔案都有確定性的唯一 ID，確保精確識別
2. **AI 通訊協議升級**：使用英文提示詞，發送 ID 資訊給 AI，並要求 AI 返回基於 ID 的匹配結果
3. **智能匹配策略**：優先使用 ID 匹配，路徑匹配作為備用方案
4. **完整檔名支援**：`MediaFile.name` 包含副檔名，支援所有檔案類型

### 💡 技術優勢
- **確定性**：相同檔案總是產生相同 ID，支援緩存和重複掃描
- **高效能**：使用標準庫 hash，微秒級運算，無額外依賴
- **低衝突率**：64-bit hash 提供極低的衝突機率
- **多語言支援**：正確處理 Unicode 檔名

### 🔧 破壞性變更
- 直接修正規格而不考慮向後兼容性，簡化實作
- `MediaFile` 結構完全重新設計
- AI 提示詞和回傳格式全面更新
- 檔案比對邏輯徹底重構

### 📈 修復效果
修復後，用戶的問題將徹底解決：
- ✅ AI 回傳的 5 個匹配對將被系統正確識別
- ✅ 系統將顯示具體的匹配結果而非 "No matching file pairs found"
- ✅ 提供詳細的除錯資訊協助問題診斷
- ✅ 支援 recursive 和非 recursive 兩種模式
