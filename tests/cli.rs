use clap::Parser;
use multi_manager::cli::CliArgs;

#[test]
fn parses_save_desktops_default() {
    let args = CliArgs::parse_from(["prog", "--save-desktops"]);
    assert_eq!(args.save_desktops.as_deref(), Some("desktop_layout.json"));
}

#[test]
fn parses_save_desktops_custom() {
    let args = CliArgs::parse_from(["prog", "--save-desktops", "foo.json"]);
    assert_eq!(args.save_desktops.as_deref(), Some("foo.json"));
}

#[test]
fn parses_load_desktops_default() {
    let args = CliArgs::parse_from(["prog", "--load-desktops"]);
    assert_eq!(args.load_desktops.as_deref(), Some("desktop_layout.json"));
}

#[test]
fn parses_load_desktops_custom() {
    let args = CliArgs::parse_from(["prog", "--load-desktops", "bar.json"]);
    assert_eq!(args.load_desktops.as_deref(), Some("bar.json"));
}

#[test]
fn parses_save_workspaces_default() {
    let args = CliArgs::parse_from(["prog", "--save-workspaces"]);
    assert_eq!(args.save_workspaces.as_deref(), Some("workspaces.json"));
}

#[test]
fn parses_save_workspaces_custom() {
    let args = CliArgs::parse_from(["prog", "--save-workspaces", "ws.json"]);
    assert_eq!(args.save_workspaces.as_deref(), Some("ws.json"));
}

#[test]
fn parses_load_workspaces_default() {
    let args = CliArgs::parse_from(["prog", "--load-workspaces"]);
    assert_eq!(args.load_workspaces.as_deref(), Some("workspaces.json"));
}

#[test]
fn parses_load_workspaces_custom() {
    let args = CliArgs::parse_from(["prog", "--load-workspaces", "ws2.json"]);
    assert_eq!(args.load_workspaces.as_deref(), Some("ws2.json"));
}

#[test]
fn parses_move_origin_flag() {
    let args = CliArgs::parse_from(["prog", "--move-origin"]);
    assert!(args.move_origin);
}

#[test]
fn parses_open_log_folder_flag() {
    let args = CliArgs::parse_from(["prog", "--open-log-folder"]);
    assert!(args.open_log_folder);
}

#[test]
fn parses_edit_settings_flag() {
    let args = CliArgs::parse_from(["prog", "--edit-settings"]);
    assert!(args.edit_settings);
}
