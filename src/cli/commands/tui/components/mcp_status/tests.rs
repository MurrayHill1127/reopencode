//! Tests for McpStatusPanel Component
//!
//! TDD test suite - all tests should FAIL initially since the component
//! is not implemented. These tests define the expected behavior.

use super::*;
use crate::mcp::types::McpStatus;
use crate::mcp::types::McpTool;
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

/// Helper to create a test terminal
fn test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 24);
    Terminal::new(backend).unwrap()
}

/// Helper to create mock tools for testing
fn create_test_tools(count: usize) -> Vec<McpTool> {
    (0..count)
        .map(|i| McpTool {
            name: format!("tool_{}", i),
            description: Some(format!("Tool number {}", i)),
            input_schema: serde_json::json!({"type": "object"}),
        })
        .collect()
}

// ============================================================================
// UNIT TESTS - Status Rendering
// ============================================================================

#[test]
fn test_mcp_status_panel_renders_connected_server() {
    // GIVEN: A panel with a connected MCP server
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("github".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    // WHEN: The panel is rendered
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 40, 10);

    // THEN: The render should complete and show "Connected" status
    // NOTE: This will panic with todo!() until implementation
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render connected server with green indicator");
}

#[test]
fn test_mcp_status_panel_renders_failed_server() {
    // GIVEN: A panel with a failed MCP server
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert(
        "broken-server".to_string(),
        McpStatus::Failed {
            error: "Connection refused".to_string(),
        },
    );
    panel.set_statuses(statuses);

    // WHEN: The panel is rendered
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 40, 10);

    // THEN: The render should complete and show "Failed" status with error
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render failed server with red indicator and error message");
}

#[test]
fn test_mcp_status_panel_renders_disabled_server() {
    // GIVEN: A panel with a disabled MCP server
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("disabled-server".to_string(), McpStatus::Disabled);
    panel.set_statuses(statuses);

    // WHEN: The panel is rendered
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 40, 10);

    // THEN: The render should complete and show "Disabled" status
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render disabled server with gray indicator");
}

#[test]
fn test_mcp_status_panel_renders_needs_auth_server() {
    // GIVEN: A panel with a server requiring authentication
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("auth-required".to_string(), McpStatus::NeedsAuth);
    panel.set_statuses(statuses);

    // WHEN: The panel is rendered
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 40, 10);

    // THEN: The render should complete and show "Needs Auth" status
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render needs-auth server with yellow indicator");
}

#[test]
fn test_mcp_status_panel_shows_tool_count() {
    // GIVEN: A panel with a connected server that has tools
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("github".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    let mut tool_counts = HashMap::new();
    tool_counts.insert("github".to_string(), 5);
    panel.set_tool_counts(tool_counts);

    // WHEN: The panel is rendered
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 40, 10);

    // THEN: The render should show the tool count (e.g., "5 tools")
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render tool count for connected server");

    // Verify tool count is accessible
    let servers = panel.servers();
    assert_eq!(servers.len(), 1);
    // Note: This assertion depends on set_tool_counts working correctly
    // The actual rendering verification would check the buffer content
}

#[test]
fn test_mcp_status_panel_status_colors() {
    // Test that status_color returns correct colors for each status

    // Connected should be green
    let connected_color = McpStatusPanel::status_color(&McpStatus::Connected);
    assert_eq!(connected_color, ratatui::style::Color::Green);

    // Failed should be red
    let failed_color = McpStatusPanel::status_color(&McpStatus::Failed {
        error: "test".to_string(),
    });
    assert_eq!(failed_color, ratatui::style::Color::Red);

    // Disabled should be gray
    let disabled_color = McpStatusPanel::status_color(&McpStatus::Disabled);
    assert_eq!(disabled_color, ratatui::style::Color::Gray);

    // NeedsAuth should be yellow
    let needs_auth_color = McpStatusPanel::status_color(&McpStatus::NeedsAuth);
    assert_eq!(needs_auth_color, ratatui::style::Color::Yellow);
}

#[test]
fn test_mcp_status_panel_footer_display() {
    // GIVEN: A panel that should display footer
    let panel = McpStatusPanel::new();

    // WHEN: We get the footer text
    let footer = McpStatusPanel::footer_text();

    // THEN: Footer should contain navigation hints
    assert!(footer.contains("↑") || footer.contains("Navigate"));
    assert!(footer.contains("Expand") || footer.contains("Enter"));
}

#[test]
fn test_mcp_status_panel_expanded_view() {
    // GIVEN: A panel with servers and expanded view
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("github".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    // WHEN: We toggle expanded view
    assert!(!panel.is_expanded());
    panel.toggle_expanded();

    // THEN: Panel should be in expanded state
    assert!(panel.is_expanded());

    // WHEN: We render in expanded mode
    let mut terminal = test_terminal();
    let area = Rect::new(0, 0, 60, 20);

    // THEN: Expanded view should show more details
    terminal
        .draw(|f| {
            panel.render(f, area);
        })
        .expect("Should render expanded view with server details");
}

// ============================================================================
// NAVIGATION TESTS
// ============================================================================

#[test]
fn test_mcp_status_panel_navigation_next() {
    // GIVEN: A panel with multiple servers
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("server-a".to_string(), McpStatus::Connected);
    statuses.insert("server-b".to_string(), McpStatus::Connected);
    statuses.insert("server-c".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    // WHEN: We navigate next
    assert_eq!(panel.selected(), 0);
    panel.select_next();
    assert_eq!(panel.selected(), 1);
    panel.select_next();
    assert_eq!(panel.selected(), 2);
    // Wrap around
    panel.select_next();
    assert_eq!(panel.selected(), 0);
}

#[test]
fn test_mcp_status_panel_navigation_prev() {
    // GIVEN: A panel with multiple servers
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("server-a".to_string(), McpStatus::Connected);
    statuses.insert("server-b".to_string(), McpStatus::Connected);
    statuses.insert("server-c".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    // WHEN: We navigate prev (wraps from 0 to last)
    assert_eq!(panel.selected(), 0);
    panel.select_prev();
    assert_eq!(panel.selected(), 2); // Wrap to last
    panel.select_prev();
    assert_eq!(panel.selected(), 1);
}

#[test]
fn test_mcp_status_panel_empty_navigation() {
    // GIVEN: A panel with no servers
    let panel = McpStatusPanel::new();

    // WHEN: We try to navigate
    // THEN: Should not panic, stay at 0
    assert_eq!(panel.selected(), 0);
}

// ============================================================================
// STATUS TEXT TESTS
// ============================================================================

#[test]
fn test_mcp_status_panel_status_text() {
    // Test that status_text returns correct strings

    assert_eq!(
        McpStatusPanel::status_text(&McpStatus::Connected),
        "Connected"
    );
    assert_eq!(
        McpStatusPanel::status_text(&McpStatus::Failed {
            error: "test".to_string()
        }),
        "Failed"
    );
    assert_eq!(
        McpStatusPanel::status_text(&McpStatus::Disabled),
        "Disabled"
    );
    assert_eq!(
        McpStatusPanel::status_text(&McpStatus::NeedsAuth),
        "Needs Auth"
    );
}

// ============================================================================
// COMPONENT TRAIT TESTS
// ============================================================================

#[test]
fn test_mcp_status_panel_is_focusable() {
    let panel = McpStatusPanel::new();
    assert!(panel.is_focusable());
}

#[test]
fn test_mcp_status_panel_focus_state() {
    let mut panel = McpStatusPanel::new();
    assert!(!panel.focused());

    panel.on_focus();
    assert!(panel.focused());

    panel.on_blur();
    assert!(!panel.focused());
}

#[test]
fn test_mcp_status_panel_unique_ids() {
    let panel1 = McpStatusPanel::new();
    let panel2 = McpStatusPanel::new();
    assert_ne!(panel1.id(), panel2.id());
}

#[test]
fn test_mcp_status_panel_default() {
    let panel = McpStatusPanel::default();
    assert!(panel.servers().is_empty());
    assert!(!panel.focused());
    assert!(!panel.is_expanded());
}

// ============================================================================
// INTEGRATION TESTS - Polling
// ============================================================================

#[test]
fn test_mcp_status_polling_updates_state() {
    // GIVEN: A panel and a mock manager with initial state
    use super::mock::MockMcpManager;

    let mut mock = MockMcpManager::new();
    mock.add_server("test-server", McpStatus::Connected);

    let mut panel = McpStatusPanel::new();
    panel.set_statuses(mock.statuses());

    // Initial state
    assert_eq!(panel.servers().len(), 1);

    // WHEN: The manager state changes (simulating a poll update)
    mock.simulate_status_change(
        "test-server",
        McpStatus::Failed {
            error: "Connection lost".to_string(),
        },
    );

    // And we update the panel
    panel.set_statuses(mock.statuses());

    // THEN: Panel should reflect the new state
    let servers = panel.servers();
    assert_eq!(servers.len(), 1);
    assert!(matches!(&servers[0].status, McpStatus::Failed { .. }));
}

#[test]
fn test_mcp_status_change_detection() {
    // GIVEN: A panel with multiple servers
    use super::mock::MockMcpManager;

    let mut mock = MockMcpManager::new();
    mock.add_server("server-1", McpStatus::Connected);
    mock.add_server("server-2", McpStatus::Connected);
    mock.add_server("server-3", McpStatus::Disabled);

    let mut panel = McpStatusPanel::new();
    panel.set_statuses(mock.statuses());
    panel.set_tool_counts(mock.tool_counts());

    // Initial count
    assert_eq!(panel.servers().len(), 3);

    // WHEN: One server's status changes
    mock.simulate_status_change("server-2", McpStatus::NeedsAuth);
    panel.set_statuses(mock.statuses());

    // THEN: We can detect which server changed
    let servers = panel.servers();
    let server_2 = servers.iter().find(|s| s.name == "server-2");
    assert!(server_2.is_some());
    assert!(matches!(server_2.unwrap().status, McpStatus::NeedsAuth));

    // Other servers unchanged
    let server_1 = servers.iter().find(|s| s.name == "server-1");
    assert!(matches!(server_1.unwrap().status, McpStatus::Connected));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_mcp_status_panel_sorted_servers() {
    // GIVEN: Servers added in random order
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert("zebra".to_string(), McpStatus::Connected);
    statuses.insert("alpha".to_string(), McpStatus::Connected);
    statuses.insert("beta".to_string(), McpStatus::Connected);
    panel.set_statuses(statuses);

    // WHEN: We get the servers
    let servers = panel.servers();

    // THEN: They should be sorted alphabetically
    assert_eq!(servers[0].name, "alpha");
    assert_eq!(servers[1].name, "beta");
    assert_eq!(servers[2].name, "zebra");
}

#[test]
fn test_mcp_status_panel_tool_count_zero_for_failed() {
    // GIVEN: A panel with a failed server
    let mut panel = McpStatusPanel::new();
    let mut statuses = HashMap::new();
    statuses.insert(
        "failed-server".to_string(),
        McpStatus::Failed {
            error: "Error".to_string(),
        },
    );
    panel.set_statuses(statuses);

    // WHEN: We check tool count
    let servers = panel.servers();

    // THEN: Tool count should be 0
    assert_eq!(servers[0].tool_count, 0);
}
