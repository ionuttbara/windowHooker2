# IBB-Hooker

A powerful and lightweight Windows desktop enhancement utility written in Rust. IBB-Hooker utilizes global Windows API hooks to extend standard window management with custom features, hotkeys, and system tweaks.

## Features

* **Advanced Window Management:**
    * **Always on Top:** Pin any window to remain above all others.
    * **Transparency:** Toggle custom transparency levels for active windows.
    * **Roll Up / Down:** Collapse windows to just their title bar to save screen space.
    * **Send to Tray:** Minimize any standard application directly to the system tray.
* **Smart Window Snapping:** Overrides the native Windows snap assist with a custom, fluid resizing overlay for adjacent snapped windows.
* **Custom Alt-Tab Overlay:** A customizable replacement for the default Windows Alt-Tab task switcher.
* **Hardware Remapping:** Intercepts and blocks the modern Windows Copilot key, seamlessly remapping it to the Right Control key.
* **System Optimization:** Includes a Standby Memory Cleaner that automatically flushes cached memory every 10 minutes or manually on demand.
* **Process Management:** Quickly terminate non-responsive or targeted applications via a global hotkey.
* **Multi-language Support:** System Tray interface available in English, Romanian, and Hungarian.

## Project Structure

The workspace is divided into several discrete Rust components to separate the background processing from the injected hooks:

* **manager (Executable):** The core background process. It handles the System Tray interface, registers global hotkeys, manages configuration state (saved in the Windows Registry), and coordinates the various enhancement modules.
* **hook (Dynamic Library):** Compiles into `hook-x64.dll` and `hook-x86.dll`. Injects into running processes via `SetWindowsHookEx` to intercept window messages (e.g., drawing custom options in application title bar context menus).
* **windowmanager (Library):** Contains the core layout and rendering logic for the Smart Snapping system and the custom Alt-Tab interface.
* **shared (Library):** A lightweight library containing shared constants, IPC message IDs, versioning, and the translation dictionary used across the manager and hooks.

## Default Key Bindings

IBB-Hooker registers the following global hotkeys for quick window manipulation:

* `Win + Ctrl + F9`: Toggle Window Transparency
* `Win + Ctrl + F10`: Toggle Always on Top
* `Win + Ctrl + F11`: Roll Up / Roll Down Window
* `Win + Ctrl + F12`: Send Window to System Tray
* `Alt + Ctrl + F4`: Force Kill Target Application

## Configuration

All features can be toggled directly via the System Tray context menu. Settings are persistently stored in the Windows Registry under the following path:
`HKEY_CURRENT_USER\Software\Gallery Inc\IBBE-Hooker`

Configurable toggles include:
* Window Snapping enable/disable
* Alt-Tab Overlay enable/disable
* Copilot Key Blocker enable/disable
* Auto-Clean Standby Memory enable/disable
* Run on Startup
* Language Selection

## Building and Installation

### Prerequisites
* Rust Toolchain (cargo, rustc)
* Windows SDK (MSVC)

### Compilation
The project requires building both the 64-bit manager and the 32-bit/64-bit hook DLLs to ensure compatibility across all Windows applications. A batch script is provided to automate this pipeline.

1. Open a terminal in the root directory of the project.
2. Execute the build script: `build.bat`
3. The final compiled binaries (`manager.exe`, `hook-x64.dll`, `hook-x86.dll`) will be output to the `Dist` directory.

### Usage
To start the application, execute `manager.exe` from the `Dist` directory. Ensure that both `hook-x64.dll` and `hook-x86.dll` remain in the same directory as the executable, as the manager dynamically loads them during runtime. To safely exit and detach the hooks from running processes, use the "Exit" option in the System Tray menu.