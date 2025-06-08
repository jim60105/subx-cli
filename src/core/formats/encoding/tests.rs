#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use crate::init_config_manager;

    #[test]
    fn test_utf8_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let text = "Hello, 世界! 🌍";
        let info = detector.detect_encoding(text.as_bytes()).unwrap();
        assert_eq!(info.charset, Charset::Utf8);
        assert!(info.confidence > 0.8);
    }

    #[test]
    fn test_utf8_with_bom_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let mut data = vec![0xEF,0xBB,0xBF];
        data.extend_from_slice(b"Hello, World!");
        let info = detector.detect_encoding(&data).unwrap();
        assert_eq!(info.charset, Charset::Utf8);
        assert_eq!(info.confidence, 1.0);
        assert!(info.bom_detected);
    }

    #[test]
    fn test_gbk_pattern_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let gbk = vec![0xC4,0xE3,0xBA,0xC3];
        let info = detector.detect_encoding(&gbk).unwrap();
        assert_ne!(info.charset, Charset::Utf8);
    }

    #[test]
    fn test_encoding_conversion() {
        let converter = EncodingConverter::new();
        let text = "測試文字";
        let res = converter.convert_to_utf8(text.as_bytes(), &Charset::Utf8).unwrap();
        assert_eq!(res.converted_text, text);
        assert!(!res.had_errors);
        assert_eq!(res.error_count, 0);
    }

    #[test]
    fn test_file_encoding_detection() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.txt");
        fs::write(&path, "Hello, 世界!").unwrap();
        let info = detector.detect_file_encoding(path.to_str().unwrap()).unwrap();
        assert_eq!(info.charset, Charset::Utf8);
    }

    #[test]
    fn test_byte_analyzer() {
        let mut analyzer = ByteAnalyzer::new();
        let data = b"Hello, World! 123";
        let result = analyzer.analyze(data).unwrap();
        assert!(result.ascii_ratio > 0.9);
        assert!(result.entropy > 0.0);
        assert!(result.control_char_ratio < 0.1);
    }

    #[test]
    fn test_unknown_encoding_fallback() {
        init_config_manager().unwrap();
        let detector = EncodingDetector::new().unwrap();
        let random: Vec<u8> = (0..100).map(|i| (i*7) as u8).collect();
        let info = detector.detect_encoding(&random).unwrap();
        assert!(info.confidence < 0.9);
    }
}
