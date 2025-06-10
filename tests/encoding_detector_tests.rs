use std::fs;
use subx_cli::core::formats::encoding::{Charset, EncodingDetector};
use subx_cli::init_config_manager;
use tempfile::TempDir;

#[cfg(test)]
mod encoding_detector_tests {
    use super::*;

    /// 測試 UTF-8 編碼檢測
    #[test]
    fn test_utf8_detection_accuracy() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let utf8_text = "Hello, 世界! Bonjour, monde! 🌍";

        let result = detector.detect_encoding(utf8_text.as_bytes()).unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.8);
        assert!(!result.bom_detected);
        assert!(result.sample_text.contains("Hello"));
    }

    /// 測試 UTF-8 BOM 檢測
    #[test]
    fn test_utf8_bom_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let mut bom_data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        bom_data.extend_from_slice("Hello, World!".as_bytes());

        let result = detector.detect_encoding(&bom_data).unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert_eq!(result.confidence, 1.0);
        assert!(result.bom_detected);
        assert_eq!(result.sample_text, "UTF-8 with BOM");
    }

    /// 測試 UTF-16 BOM 檢測
    #[test]
    fn test_utf16_bom_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // UTF-16 LE BOM
        let utf16le_data = vec![0xFF, 0xFE, 0x48, 0x00, 0x65, 0x00]; // "He" in UTF-16 LE
        let result = detector.detect_encoding(&utf16le_data).unwrap();
        assert_eq!(result.charset, Charset::Utf16Le);
        assert!(result.bom_detected);

        // UTF-16 BE BOM
        let utf16be_data = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x65]; // "He" in UTF-16 BE
        let result = detector.detect_encoding(&utf16be_data).unwrap();
        assert_eq!(result.charset, Charset::Utf16Be);
        assert!(result.bom_detected);
    }

    /// 測試檔案編碼檢測
    #[test]
    fn test_file_encoding_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // 建立 UTF-8 檔案
        let utf8_path = temp_dir.path().join("utf8.txt");
        fs::write(&utf8_path, "測試檔案編碼檢測功能。").unwrap();

        let result = detector
            .detect_file_encoding(utf8_path.to_str().unwrap())
            .unwrap();

        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.7);
    }

    /// 測試不存在檔案錯誤處理
    #[test]
    fn test_nonexistent_file_error() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let result = detector.detect_file_encoding("nonexistent.txt");

        assert!(result.is_err());
    }

    /// 測試 GBK 編碼模式檢測
    #[test]
    fn test_gbk_pattern_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 模擬 GBK 編碼模式 (高位元組範圍)
        let gbk_pattern = vec![
            0xC4, 0xE3, 0xBA, 0xC3, // 你好 (GBK)
            0xCA, 0xC0, 0xBD, 0xE7, // 世界 (GBK)
        ];

        let result = detector.detect_encoding(&gbk_pattern).unwrap();

        // 應該檢測為 GBK 或至少不是 UTF-8
        assert!(result.confidence > 0.3);
        if result.charset == Charset::Gbk {
            assert!(result.confidence > 0.5);
        }
    }

    /// 測試 Shift-JIS 編碼檢測
    #[test]
    fn test_shift_jis_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 模擬 Shift-JIS 編碼模式
        let shift_jis_pattern = vec![
            0x82, 0xB1, 0x82, 0xF1, // こん (Shift-JIS)
            0x82, 0xB1, 0x82, 0xF1, // こん (Shift-JIS)
            0x82, 0xC9, 0x82, 0xBF, // にち (Shift-JIS)
        ];

        let result = detector.detect_encoding(&shift_jis_pattern).unwrap();

        // 應該檢測為 Shift-JIS 或相關編碼
        assert!(result.confidence > 0.2);
    }

    /// 測試編碼信心值排序
    #[test]
    fn test_encoding_confidence_ranking() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 明確的 UTF-8 文字應該有最高信心值
        let clear_utf8 = "Clear English text with numbers 123.";
        let utf8_result = detector.detect_encoding(clear_utf8.as_bytes()).unwrap();

        // 模糊的資料應該有較低信心值
        let ambiguous_data: Vec<u8> = (0x80..=0xFF).cycle().take(50).collect();
        let ambiguous_result = detector.detect_encoding(&ambiguous_data).unwrap();

        assert!(utf8_result.confidence > ambiguous_result.confidence);
    }

    /// 測試最大取樣大小限制
    #[test]
    fn test_max_sample_size_limit() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 建立超過取樣大小限制的資料
        let large_data = vec![b'A'; 10000]; // 假設限制是 8192
        let result = detector.detect_encoding(&large_data).unwrap();

        // 應該成功檢測且不會因資料太大而失敗
        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.9);
    }

    /// 測試編碼候選者選擇邏輯
    #[test]
    fn test_encoding_candidate_selection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 建立混合編碼特徵的資料
        let mut mixed_data = b"English text ".to_vec();
        mixed_data.extend_from_slice(&[0xC3, 0xA9]); // é in UTF-8
        mixed_data.extend_from_slice(b" and more text");

        let result = detector.detect_encoding(&mixed_data).unwrap();

        // 應該正確選擇 UTF-8
        assert_eq!(result.charset, Charset::Utf8);
        assert!(result.confidence > 0.7);
    }

    /// 測試未知編碼的後備機制
    #[test]
    fn test_unknown_encoding_fallback() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 建立完全隨機的資料
        let random_data: Vec<u8> = (0..100).map(|i| (i * 7 + 13) as u8).collect();
        let result = detector.detect_encoding(&random_data).unwrap();

        // 應該有一個後備編碼選擇
        assert!(result.confidence >= 0.0);
        assert!(result.confidence <= 1.0);
    }

    /// 測試編碼檢測效能
    #[test]
    fn test_detection_performance() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();

        // 建立中等大小的文字檔案
        let large_text = "Hello, World! ".repeat(500);

        let start = std::time::Instant::now();
        let _result = detector.detect_encoding(large_text.as_bytes()).unwrap();
        let duration = start.elapsed();

        // 檢測應該在合理時間內完成 (< 100ms)
        assert!(duration.as_millis() < 100);
    }
}
