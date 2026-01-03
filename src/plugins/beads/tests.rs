//! Beads plugin tests
use super::*;

#[test]
fn test_beads_plugin_creation() {
    let plugin = BeadsPlugin::new();
    assert_eq!(plugin.id(), "beads");
    assert_eq!(plugin.name(), "Beads Issue Tracker");
    assert!(plugin.capabilities().has_menu);
    assert!(plugin.capabilities().has_status);
}

#[test]
fn test_beads_plugin_menu_item() {
    let plugin = BeadsPlugin::new();
    let menu = plugin.menu_item().unwrap();
    assert_eq!(menu.key, 'B');
    assert_eq!(menu.name, "Beads");
}
