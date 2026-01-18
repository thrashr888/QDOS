//! Plugin system tests
use super::*;
use qdos_plugin_api::PluginCapabilities;
use std::any::Any;
use std::collections::HashMap;

struct TestPlugin {
    id: String,
}

impl TestPlugin {
    fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl Plugin for TestPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Test Plugin"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            ..Default::default()
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Test".to_string(),
            key: 'T',
            description: "Test plugin".to_string(),
            priority: 100,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[test]
fn test_plugin_manager() {
    let mut manager = PluginManager::new();
    manager.register(Box::new(TestPlugin::new("test")));

    assert!(manager.get("test").is_some());
    assert!(manager.get("nonexistent").is_none());

    let menu_items = manager.menu_plugins();
    assert_eq!(menu_items.len(), 1);
    assert_eq!(menu_items[0].1.name, "Test");
}

#[test]
fn test_plugin_manager_with_config() {
    // Create a config that disables the "test" plugin
    let config = PluginsConfig {
        enabled: vec![],
        disabled: vec!["test".to_string()],
        settings: HashMap::new(),
    };

    let mut manager = PluginManager::with_config(config);
    manager.register(Box::new(TestPlugin::new("test")));

    // Plugin should not be registered because it's disabled
    assert!(manager.get("test").is_none());
}

#[test]
fn test_plugin_manager_register_always() {
    // Create a config that disables the "test" plugin
    let config = PluginsConfig {
        enabled: vec![],
        disabled: vec!["test".to_string()],
        settings: HashMap::new(),
    };

    let mut manager = PluginManager::with_config(config);
    // register_always ignores config
    manager.register_always(Box::new(TestPlugin::new("test")));

    // Plugin should be registered because we used register_always
    assert!(manager.get("test").is_some());
}
