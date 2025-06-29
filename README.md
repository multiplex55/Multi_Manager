# Multi Manager

Multi Manager is a robust workspace and window management application built using Rust. It provides an intuitive GUI to manage multiple workspaces, allowing users to capture, organize, and manipulate application windows. This tool is ideal for power users, developers, and anyone looking to optimize their multitasking workflow.

https://github.com/user-attachments/assets/452cc353-c795-428a-a3e7-dca2cd9c3ce0


- **In Development**: Note that this project is still in heavy development and subject to change. Features may be different than what appears in the video.Feedback appreciated.

---

## Features

- **Workspace Management**: Create, rename, and delete workspaces.
- **Window Management**:
  - Capture active windows and associate them with specific workspaces.
  - Save "Home" and "Target" window positions.
  - Move windows between "Home" and "Target" positions.
  - Right-click a valid window and choose **Force Move to Origin** to center it on the main desktop.
- **Hotkey Support**: Assign global hotkeys to workspaces for quick activation.
- **Hotkey Display**: The workspace title shows the assigned hotkey (e.g., `Workspace 1 - F13`) and updates automatically when you rename the workspace or change its hotkey.
- **Validation System**:
  - Validate hotkey configurations at startup and during updates.
  - Indicate the validity of hotkeys in real-time.
- **Valid Window Filtering**:
  - Only valid windows (as determined by `IsWindow`) are considered for operations.
  - Invalid windows are ignored, preventing unnecessary errors.
- **Persistent Storage**:
  - Save and load workspace configurations in JSON format.
  - Pretty-printed JSON for easy manual editing.
  - Optional auto-save to persist changes automatically.
- **Desktop Management**:
  - Save and restore window layouts across all virtual desktops from the **File -> Desktop Management** menu.
  - Move all windows back to their original monitors with the **Move All to Origin** function.
- **Visual Feedback**:
  - Color-coded HWND validity indicators for associated windows.
  - Popup dialogs for feedback (e.g., workspace saved successfully).
- **Customization**: Easily extendable code for additional features.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)

---

## How It Works

### GUI Overview

- The application uses [eframe](https://github.com/emilk/eframe) for the GUI.
- Workspaces are displayed as collapsible sections.
- Each workspace can hold multiple captured windows with associated metadata.
- The main interface includes:
  - Buttons for workspace and window operations.
  - Hotkey validation indicators.
  - Window position management tools.

### Backend Functionality

- **Workspace Struct**:
  Stores information about each workspace, including name, hotkey, and associated windows.
- **Window Struct**:
  Stores metadata for individual application windows, such as HWND, title, and positions.
- **Utils Module**:
  Includes utility functions like `show_message_box` for displaying dialog boxes.
- **Hotkey Management**:
  Uses the `windows` crate to register, validate, and handle global hotkeys.
- **Window Validity Filtering**:
  Filters invalid windows during operations like position checks and toggling.

---

## Getting Started

### Prerequisites

- **Rust**: Install [Rust](https://www.rust-lang.org/tools/install) to build and run the project.
- **Windows OS**: This application leverages the Windows API and is not compatible with other operating systems.

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/multi-manager.git
   cd multi-manager
   ```

2. Install dependencies:
   ```bash
   cargo build
   ```

3. Run the application:
   ```bash
   multi-manager
   ```

---

## Usage

### Workspace Management

1. **Create Workspace**: Use the "Add New Workspace" button to create a new workspace.
2. **Rename Workspace**:
   - Right-click the workspace header to open the rename dialog.
   - Enter a new name and confirm by clicking "Ok."
3. **Delete Workspace**: Click "Delete Workspace" to remove a workspace.
4. **Save Workspaces As...**: Choose **File -> Workspace Management -> Save Workspaces As...** to pick a custom JSON file. The selected path is remembered for future saves.

### Window Management

1. **Capture Active Window**: Select "Capture Active Window" to add the current window to the selected workspace.
2. **Set Positions**:
   - Use "Capture Home" or "Capture Target" to record window positions.
   - Adjust positions using the provided drag values.
3. **Move Windows**:
   - "Move to Home" relocates the window to its recorded home position.
   - "Move to Target" relocates the window to its target position.
4. **Valid Window Filtering**:
   - Only valid windows (as determined by the `IsWindow` API) are displayed and operated on.
   - Invalid windows are marked with a red indicator and ignored during toggles or moves.

5. **Force Move to Origin**: Right-click a valid window to center it on the main desktop.
### Hotkey Management

1. **Assign Hotkeys**:
   - Enter a valid hotkey combination in the input field.
   - Click "Validate Hotkey" to confirm.
2. **Activate Workspace**: Use the assigned hotkey to activate the workspace and toggle window positions.

### Desktop Management

1. Open **File -> Desktop Management**.
2. Choose **Save All Desktops** to store the current window layout.
3. Choose **Restore All Desktops** to reload the saved layout.
4. Select **File -> Desktop Management -> Move All to Origin**. Confirm the prompt, and a completion message will appear once all windows are centered.

### Command Line Examples

Run the application with optional arguments:

```bash
# Display available flags
multi-manager --help

# Save or load desktop layouts (defaults to desktop_layout.json)
multi-manager --save-desktops
multi-manager --save-desktops custom_layout.json
multi-manager --load-desktops
multi-manager --load-desktops custom_layout.json

# Save or load workspace data (defaults to workspaces.json)
multi-manager --save-workspaces
multi-manager --save-workspaces my_workspaces.json
multi-manager --load-workspaces
multi-manager --load-workspaces my_workspaces.json

# Utility commands
multi-manager --move-origin       # centers every visible window
multi-manager --open-log-folder   # opens the folder with multi_manager.log
multi-manager --edit-settings     # opens settings.json in a text editor
```

Saving or loading prints messages like `Saved desktops to desktop_layout.json` or
`Loaded workspaces from my_workspaces.json`. The move-origin command prompts for
confirmation and shows a completion dialog. The log and settings commands open
Explorer or your editor without additional console output.

---

## Configuration

### Workspace Storage

- Workspaces are saved in `workspaces.json` by default.
- Use **Save Workspaces As...** to choose another location; the last path is stored in `settings.json`.
- The file uses a pretty-printed JSON format for easy manual edits.

---

## Compatibility

- **Operating System**: Windows 10 or later.
- **Rust Version**: Requires the latest stable Rust compiler.

---

## How It Works

### Validation System

- Hotkeys are validated against a regex to ensure compatibility.
- Initial validation runs once at startup for all saved workspaces.

### HWND Validity

- Checks if window handles (HWND) are still valid using the `IsWindow` API.
- Displays results with color-coded indicators:
  - Green: Valid HWND.
  - Red: Invalid HWND.

### Persistent Storage

- Workspaces are saved to `workspaces.json` whenever changes are made.
- Upon startup, the JSON file is loaded to restore previous configurations.

---

## Contributing

1. Fork the repository.
2. Create a feature branch:
   ```bash
   git checkout -b feature-name
   ```
3. Commit your changes:
   ```bash
   git commit -m "Add new feature"
   ```
4. Push the branch:
   ```bash
   git push origin feature-name
   ```
5. Open a pull request.

---

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.

---

## Acknowledgments

- [eframe](https://github.com/emilk/eframe): For the GUI framework.
- [Windows API](https://learn.microsoft.com/en-us/windows/win32/api/): For system-level operations.

---

## Troubleshooting

### Common Errors

1. **HWND Not Valid**:
   - Ensure the application window is active when capturing.
   - Check if the application has proper permissions.
2. **Hotkey Not Working**:
   - Validate the hotkey combination.
   - Ensure no other application is using the same hotkey.

### Logging

- Logs are stored in `log4rs.yaml`-configured files.
- Adjust logging levels for detailed debugging.

---

