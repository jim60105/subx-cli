//! 音訊處理系統遷移策略

/// 遷移階段標記
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MigrationStage {
    /// 使用舊系統
    Legacy,
    /// 新舊系統並存
    Hybrid,
    /// 完全使用 aus
    AusOnly,
}

/// 遷移配置
pub struct MigrationConfig {
    pub stage: MigrationStage,
    pub enable_performance_comparison: bool,
    pub fallback_to_legacy: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            stage: MigrationStage::Hybrid,
            enable_performance_comparison: true,
            fallback_to_legacy: true,
        }
    }
}
