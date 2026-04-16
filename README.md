# IBB-Hooker

A powerful and lightweight Windows desktop enhancement utility written in Rust. IBB-Hooker utilizes global Windows API hooks to extend standard window management with custom features, hotkeys, and system tweaks.

## Features

* **Advanced Window Management:**
    * **Always on Top:** Pin any window to remain above all others with a customizable accent-colored border highlight.
    * **Transparency:** Toggle custom transparency levels (~80% opacity) for active windows.
    * **Roll Up / Down:** Collapse windows to just their title bar to save screen space.
    * **Send to Tray:** Minimize any standard application directly to the system tray.
* **FancyZones:** A powerful window tiling manager that allows users to define and draw custom screen regions. Snapping to these zones is triggered by holding `Shift + F1` while dragging a window.
* **Smart Window Snapping:** Overrides the native Windows snap assist with a custom resizing overlay for adjacent snapped windows.
* **Custom Alt-Tab Overlay:** A customizable replacement for the default Windows Alt-Tab task switcher.
* **Hotkey Manager:** A dedicated interface to manage and remap keys from `F1` through `F24`. Actions include running applications, simulating complex keystrokes, or triggering internal commands like "Toggle Always on Top".
* **Hardware Remapping:** Intercepts the modern Windows Copilot key, remapping it to the Right Control key or a custom executable/URL.
* **System Optimization:**
    * **Standby Memory Cleaner:** Automatically flushes cached "Standby" memory every 10 minutes or manually on demand to improve system responsiveness.
    * **Process Management:** Quickly terminate non-responsive or targeted applications via a global hotkey.
    * **Explorer Fix:** Automatically applies a full-screen toggle (F11) fix to new Windows Explorer windows to ensure the UI initializes correctly.
* **Modern UI:** Features support for Windows 10/11 Dark Mode context menus.
* **Multi-language Support:** System interface available in English, Romanian, and Hungarian.

## Project Structure

The workspace is divided into several discrete Rust components:

* **manager (Executable):** The core background process. It handles the System Tray interface, registers global hotkeys, manages configuration state (stored in the Windows Registry), and coordinates the various enhancement modules.
* **hook (Dynamic Library):** Compiles into `hook-x64.dll` and `hook-x86.dll`. Injects into running processes via `SetWindowsHookEx` to intercept window messages.
* **windowmanager (Library):** Contains the core logic for the Smart Snapping system, custom Alt-Tab interface, and the **FancyZones** manager.
* **shared (Library):** A lightweight library containing shared constants, IPC message IDs, versioning, and the translation dictionary.

## Default Key Bindings

IBB-Hooker registers the following global hotkeys:

* `Win + Ctrl + F9`: Toggle Window Transparency
* `Win + Ctrl + F10`: Toggle Always on Top
* `Win + Ctrl + F11`: Roll Up / Roll Down Window
* `Win + Ctrl + F12`: Send Window to System Tray
* `Alt + Ctrl + F4`: Force Kill Target Application
* `Shift + F1`: (While dragging) Snap window to FancyZones

## Configuration

All features can be toggled via the System Tray context menu. Settings are persistently stored in the Windows Registry under:
`HKEY_CURRENT_USER\Software\Gallery Inc\IBBE-Hooker`

Specific configurations include:
* **FancyZones Layouts:** `...\IBBE-Hooker\FancyZones`
* **Custom Hotkeys:** `...\IBBE-Hooker\Hotkeys`
* **Copilot Action:** `...\IBBE-Hooker\NoCopilot`

## Building and Installation

### Prerequisites
* Rust Toolchain (cargo, rustc)
* Windows SDK (MSVC)

### Compilation
The project requires building both the 64-bit manager and the 32-bit/64-bit hook DLLs to ensure compatibility across all applications.

1.  Open a terminal in the root directory.
2.  Execute the build script: `build.bat`.
3.  The final compiled binaries (`manager.exe`, `hook-x64.dll`, `hook-x86.dll`) will be output to the `Dist` directory.

### Usage
To start the application, execute `manager.exe` from the `Dist` directory. Ensure that both `hook-x64.dll` and `hook-x86.dll` remain in the same directory as the executable, as the manager dynamically loads them. To safely exit and detach hooks, use the "Exit" option in the System Tray menu.