#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::source::{FileSource, EnvSource, ArgsSource};
    use crate::config::manager::ConfigManager;
    use crate::config::manager::ConfigError;
    use crate::config::partial::PartialConfig;
    use crate::config::validator::{AIConfigValidator, SyncConfigValidator};
    use crate::config::cache::ConfigCache;
    use std::fs;
    use tempfile::tempdir;
    use std::env;

    #[test]
    fn file_source_load_partial_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.toml");
        let content = r#"
[ai]
model = "unit-test-model"
temperature = 1.23

[formats]
default_output = "vtt"
"#;
        fs::write(&file_path, content).unwrap();
        let source = FileSource::new(file_path.clone());
        let partial = source.load().unwrap();
        assert_eq!(partial.ai.model, Some("unit-test-model".into()));
        assert_eq!(partial.ai.temperature, Some(1.23));
        assert_eq!(partial.formats.default_output, Some("vtt".into()));
    }

    #[test]
    fn env_source_load_partial_config() {
        let prefix = "TEST_";
        env::set_var("TEST_OPENAI_API_KEY", "env-key-xyz");
        env::set_var("TEST_AI_MODEL", "env-model-abc");
        let source = EnvSource::new(prefix.to_string());
        let partial = source.load().unwrap();
        assert_eq!(partial.ai.api_key, Some("env-key-xyz".into()));
        assert_eq!(partial.ai.model, Some("env-model-abc".into()));
        env::remove_var("TEST_OPENAI_API_KEY");
        env::remove_var("TEST_AI_MODEL");
    }

    #[test]
    fn args_source_load_empty_partial_by_default() {
        // Currently ArgsSource does not map any fields if no CLI flags provided
        let args = crate::cli::ConfigArgs {
            action: crate::cli::ConfigAction::List,
        };
        let source = ArgsSource::new(args);
        let partial = source.load().unwrap();
        let default = PartialConfig::default();
        assert_eq!(partial.ai.provider, default.ai.provider);
        assert_eq!(partial.ai.api_key, default.ai.api_key);
        assert_eq!(partial.general.backup_enabled, default.general.backup_enabled);
    }

    #[test]
    fn config_manager_merges_sources_by_priority() {
        // two sources: low priority then high priority override
        struct Low;
        impl crate::config::source::ConfigSource for Low {
            fn load(&self) -> Result<PartialConfig, ConfigError> {
                let mut c = PartialConfig::default();
                c.general.backup_enabled = Some(false);
                Ok(c)
            }
            fn priority(&self) -> u8 { 10 }
            fn source_name(&self) -> &'static str { "low" }
        }
        struct High;
        impl crate::config::source::ConfigSource for High {
            fn load(&self) -> Result<PartialConfig, ConfigError> {
                let mut c = PartialConfig::default();
                c.general.backup_enabled = Some(true);
                Ok(c)
            }
            fn priority(&self) -> u8 { 0 }
            fn source_name(&self) -> &'static str { "high" }
        }
        let manager = ConfigManager::new()
            .add_source(Box::new(Low))
            .add_source(Box::new(High));
        manager.load().unwrap();
        let cfg = manager.config().read().unwrap();
        assert_eq!(cfg.general.backup_enabled, Some(true));
    }

    #[test]
    fn config_cache_set_get_and_cleanup() {
        let mut cache = ConfigCache::new();
        cache.set("k".into(), 42u32, Some(std::time::Duration::from_secs(0)));
        assert_eq!(cache.get::<u32>("k"), None);
        cache.set("k".into(), 100u32, None);
        assert_eq!(cache.get::<u32>("k"), Some(100u32));
        cache.clear();
        assert_eq!(cache.get::<u32>("k"), None);
    }

    #[test]
    fn ai_config_validator_rejects_invalid() {
        let mut full = crate::config::Config::default();
        full.ai.api_key = Some("wrong-key".into());
        full.ai.model = "invalid".into();
        full.ai.temperature = -1.0;
        let v = AIConfigValidator;
        assert!(v.validate(&full).is_err());
    }

    #[test]
    fn sync_config_validator_rejects_invalid() {
        let mut full = crate::config::Config::default();
        full.sync.max_offset_seconds = 0.0;
        full.sync.correlation_threshold = 2.0;
        let v = SyncConfigValidator;
        assert!(v.validate(&full).is_err());
    }
}
