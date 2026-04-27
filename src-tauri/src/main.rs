// src-tauri/src/main.rs
// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tharsis_lib::run()
}
