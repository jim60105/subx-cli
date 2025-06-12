mod common;
use common::{
    mock_openai_helper::MockOpenAITestHelper, test_data_generators::MatchResponseGenerator,
};

use subx_cli::config::TestConfigBuilder;
use subx_cli::config::service::ConfigService;
/// 基本的 Wiremock 整合測試範例，示範如何使用 MockOpenAITestHelper 及 TestConfigBuilder
#[tokio::test]
async fn wiremock_basic_integration_example() {
    // 啟動 mock OpenAI 服務並模擬成功回應
    let mock = MockOpenAITestHelper::new().await;
    mock.mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
        .await;

    // 建立 TestConfigService，將 AI base_url 指向 mock server
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock.base_url())
        .build_service();

    // 確認配置中的 base_url 已正確設置
    let config = config_service.get_config().expect("Get config");
    assert_eq!(config.ai.base_url, mock.base_url());
}
