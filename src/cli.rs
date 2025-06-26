use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about = "Multi Manager window tool", long_about = None)]
pub struct CliArgs {
    #[arg(long = "save-desktops", default_missing_value = "desktop_layout.json", num_args = 0..=1)]
    pub save_desktops: Option<String>,

    #[arg(long = "load-desktops", default_missing_value = "desktop_layout.json", num_args = 0..=1)]
    pub load_desktops: Option<String>,

    #[arg(long = "move-origin", action = ArgAction::SetTrue)]
    pub move_origin: bool,

    #[arg(long = "save-workspaces", default_missing_value = "workspaces.json", num_args = 0..=1)]
    pub save_workspaces: Option<String>,

    #[arg(long = "load-workspaces", default_missing_value = "workspaces.json", num_args = 0..=1)]
    pub load_workspaces: Option<String>,

    #[arg(long = "open-log-folder", action = ArgAction::SetTrue)]
    pub open_log_folder: bool,

    #[arg(long = "edit-settings", action = ArgAction::SetTrue)]
    pub edit_settings: bool,
}
