//! Service container for dependency management and injection.
//!
//! This module provides a centralized service container that manages
//! the lifecycle of services and components, enabling clean dependency
//! injection throughout the application.

use crate::{Result, config::ConfigService, core::ComponentFactory};
use std::sync::Arc;

/// Service container for dependency injection and service management.
///
/// The service container holds references to core services and provides
/// a centralized way to access them throughout the application. It manages
/// the lifecycle of services and ensures proper dependency injection.
///
/// # Design Principles
///
/// - **Single Source of Truth**: All services are managed through the container
/// - **Dependency Injection**: Components receive dependencies explicitly
/// - **Configuration Isolation**: Services are decoupled from global configuration
/// - **Test Friendliness**: Easy to mock and test individual components
///
/// # Examples
///
/// ```rust
/// use subx_cli::core::ServiceContainer;
/// use subx_cli::config::ProductionConfigService;
/// use std::sync::Arc;
///
/// # async fn example() -> subx_cli::Result<()> {
/// let config_service = Arc::new(ProductionConfigService::new()?);
/// let container = ServiceContainer::new(config_service)?;
///
/// // Access services through container
/// let config_service = container.config_service();
/// let factory = container.component_factory();
/// # Ok(())
/// # }
/// ```
pub struct ServiceContainer {
    config_service: Arc<dyn ConfigService>,
    component_factory: ComponentFactory,
}

impl ServiceContainer {
    /// Create a new service container with the given configuration service.
    ///
    /// # Arguments
    ///
    /// * `config_service` - Configuration service implementation
    ///
    /// # Errors
    ///
    /// Returns an error if component factory creation fails.
    pub fn new(config_service: Arc<dyn ConfigService>) -> Result<Self> {
        let component_factory = ComponentFactory::new(config_service.as_ref())?;

        Ok(Self {
            config_service,
            component_factory,
        })
    }

    /// Get a reference to the configuration service.
    ///
    /// Returns a reference to the configuration service managed by this container.
    pub fn config_service(&self) -> &Arc<dyn ConfigService> {
        &self.config_service
    }

    /// Get a reference to the component factory.
    ///
    /// Returns a reference to the component factory that can create
    /// configured instances of core components.
    pub fn component_factory(&self) -> &ComponentFactory {
        &self.component_factory
    }

    /// Reload all services and components.
    ///
    /// This method triggers a reload of the configuration service and
    /// recreates the component factory with the updated configuration.
    /// This is useful for dynamic configuration updates.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration reloading or factory recreation fails.
    pub fn reload(&mut self) -> Result<()> {
        // Reload configuration service
        self.config_service.reload()?;

        // Recreate component factory with updated configuration
        self.component_factory = ComponentFactory::new(self.config_service.as_ref())?;

        Ok(())
    }

    /// Create a new service container for testing with custom configuration.
    ///
    /// This method is useful for testing scenarios where you need to provide
    /// specific configuration values.
    ///
    /// # Arguments
    ///
    /// * `config_service` - Test configuration service
    ///
    /// # Errors
    ///
    /// Returns an error if container creation fails.
    #[cfg(test)]
    pub fn new_for_testing(config_service: Arc<dyn ConfigService>) -> Result<Self> {
        Self::new(config_service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_service::TestConfigService;

    #[test]
    fn test_service_container_creation() {
        let config_service = Arc::new(TestConfigService::default());
        let container = ServiceContainer::new(config_service);
        assert!(container.is_ok());
    }

    #[test]
    fn test_service_container_access() {
        let config_service = Arc::new(TestConfigService::default());
        let container = ServiceContainer::new(config_service.clone()).unwrap();

        // Test that we can access services
        let _retrieved_config_service = container.config_service();
        // Note: Can't test pointer equality due to trait object casting

        let factory = container.component_factory();
        assert!(factory.config().ai.provider == "openai");
    }

    #[test]
    fn test_service_container_reload() {
        let config_service = Arc::new(TestConfigService::default());
        let mut container = ServiceContainer::new(config_service).unwrap();

        // Test reload operation
        let result = container.reload();
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_for_testing() {
        let config_service = Arc::new(TestConfigService::default());
        let container = ServiceContainer::new_for_testing(config_service);
        assert!(container.is_ok());
    }
}
