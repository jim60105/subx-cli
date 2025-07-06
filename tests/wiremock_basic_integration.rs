mod common;
use common::{
    mock_openai_helper::MockOpenAITestHelper, test_data_generators::MatchResponseGenerator,
};

use subx_cli::config::TestConfigBuilder;
use subx_cli::config::service::ConfigService;
/// Basic Wiremock integration test example, demonstrating how to use MockOpenAITestHelper and TestConfigBuilder
#[tokio::test]
async fn wiremock_basic_integration_example() {
    // Start mock OpenAI service and simulate successful response
    let mock = MockOpenAITestHelper::new().await;
    mock.mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
        .await;

    // Create TestConfigService, point AI base_url to mock server
    let config_service = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_mock_ai_server(&mock.base_url())
        .build_service();

    // Verify base_url in configuration is correctly set
    let config = config_service.get_config().expect("Get config");
    assert_eq!(config.ai.base_url, mock.base_url());
}
