# Spec Delta

## ADDED Requirements

### Requirement: The Full-Workflow Test Is Hermetic

A test that spawns the CLI for the match → convert → sync workflow SHALL run with no hosted-AI credential, no outbound network and no dependency on the developer's user configuration. It SHALL give every child a temporary configuration file named by `SUBX_CONFIG_PATH`, a loopback mock server as the match endpoint, and an environment with the ambient AI provider, output-mode and proxy overrides removed.

#### Scenario: The dry-run match runs against a loopback mock

- **GIVEN** the full-workflow test starts a loopback mock serving `POST /v1/chat/completions` and writes a temporary configuration selecting the `local` provider at that server's `/v1` base URL
- **WHEN** it spawns `match --dry-run` with no hosted-provider credentials and no route to a public AI endpoint
- **THEN** the command SHALL exit successfully, print the `Preview mode` line, and the mock SHALL have received exactly one request for `/v1/chat/completions`

#### Scenario: An ambient override cannot reach any leg

- **GIVEN** a shell exporting `OPENAI_API_KEY`, `SUBX_AI_BASE_URL`, `SUBX_OUTPUT` or `LOCAL_LLM_BASE_URL`
- **WHEN** the test builds the child environment for any of its three legs
- **THEN** those variables SHALL be absent from it, and the temporary configuration file SHALL remain the only AI source

#### Scenario: A developer config file cannot fail the non-AI legs

- **GIVEN** an invalid configuration file at the platform user-config path
- **WHEN** the convert and sync legs run
- **THEN** they SHALL succeed, because `SUBX_CONFIG_PATH` names the test's own file

#### Scenario: A mock response survives a change to file IDs

- **GIVEN** a mock that builds its match response from the file IDs present in the request prompt
- **WHEN** the matcher's ID format or generation changes
- **THEN** the test SHALL still pass, because neither the expected response nor the config file contains a hard-coded ID