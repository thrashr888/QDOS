//! Git plugin tests
use super::*;

#[test]
fn test_git_plugin_creation() {
    let plugin = GitPlugin::new();
    assert_eq!(plugin.id(), "git");
    assert_eq!(plugin.name(), "Git Integration");
    assert!(plugin.capabilities().has_menu);
    assert!(plugin.capabilities().has_status);
}

#[test]
fn test_git_plugin_menu_item() {
    let plugin = GitPlugin::new();
    let menu = plugin.menu_item().unwrap();
    assert_eq!(menu.key, 'G');
    assert_eq!(menu.name, "Git");
}
