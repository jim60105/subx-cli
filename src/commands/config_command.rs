use crate::cli::{ConfigAction, ConfigArgs};
use crate::config::Config;
use crate::error::SubXError;
use crate::Result;

/// Config 子命令執行
pub async fn execute(args: ConfigArgs) -> Result<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            let parts: Vec<&str> = key.splitn(2, '.').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "ai" => match parts[1] {
                        "api_key" => config.ai.api_key = Some(value),
                        "model" => config.ai.model = value,
                        _ => {
                            return Err(SubXError::config(format!("無效的 AI 配置鍵: {}", key)));
                        }
                    },
                    "formats" => match parts[1] {
                        "default_output" => config.formats.default_output = value,
                        _ => {
                            return Err(SubXError::config(format!(
                                "無效的 Formats 配置鍵: {}",
                                key
                            )));
                        }
                    },
                    _ => {
                        return Err(SubXError::config(format!("無效的配置區段: {}", parts[0])));
                    }
                }
            } else {
                return Err(SubXError::config(format!(
                    "無效的配置鍵格式: {} (應為 section.field)",
                    key
                )));
            }
            config.save()?;
            println!("設定 {} = {}", key, config.get_value(&key)?);
        }
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            let value = config.get_value(&key)?;
            println!("{}", value);
        }
        ConfigAction::List => {
            let config = Config::load()?;
            if let Some(path) = &config.loaded_from {
                println!("# 配置檔案路徑: {}\n", path.display());
            }
            println!(
                "{}",
                toml::to_string_pretty(&config)
                    .map_err(|e| SubXError::config(format!("TOML 序列化錯誤: {}", e)))?
            );
        }
        ConfigAction::Reset => {
            let default_config = Config::default();
            default_config.save()?;
            println!("配置已重置為預設值");
            if let Ok(path) = Config::config_file_path() {
                println!("預設配置已儲存至: {}", path.display());
            }
        }
    }
    Ok(())
}
