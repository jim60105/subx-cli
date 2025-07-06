/// Convenient macro for integration tests
#[macro_export]
macro_rules! test_with_mock_ai {
    ($test_name:ident, $response:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let root = temp_dir.path();

            let mock_helper = $crate::common::MockOpenAITestHelper::new().await;
            mock_helper.mock_chat_completion_success($response).await;

            let config_service = subx_cli::config::TestConfigBuilder::new()
                .with_ai_provider("openai")
                .with_mock_ai_server(mock_helper.base_url())
                .build_service();

            $test_body(root, config_service, mock_helper).await;
        }
    };
}

/// Create integration test with error response
#[macro_export]
macro_rules! test_with_mock_ai_error {
    ($test_name:ident, $status:expr, $error_msg:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let root = temp_dir.path();

            let mock_helper = $crate::common::MockOpenAITestHelper::new().await;
            mock_helper.setup_error_response($status, $error_msg).await;

            let config_service = subx_cli::config::TestConfigBuilder::new()
                .with_ai_provider("openai")
                .with_mock_ai_server(mock_helper.base_url())
                .build_service();

            $test_body(root, config_service, mock_helper).await;
        }
    };
}
