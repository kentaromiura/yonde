// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{fs, path::PathBuf};

use rusqlite::{named_params, Connection};

// definition comes from https://jitendex.org/pages/downloads.html ( https://creativecommons.org/licenses/by-sa/4.0/)
// MDict version, db has been generated via a slightly modified https://github.com/zhimoe/mdict-rs
#[tauri::command]
fn definition(handle: tauri::AppHandle, word: String) -> String {
    let db_path = handle
        .path_resolver()
        .resolve_resource("src/jitendex.mdx.db")
        .expect("failed to resolve resource.");

    let mut def = query_internal(word.trim().to_string(), db_path.clone());
    while def.contains("@@@LINK=") {
        let new_query = def
            .replace("@@@LINK=", "")
            .replace("\x00", "")
            .trim()
            .to_string();
        def = query_internal(new_query, db_path.clone());
    }
    return def;
}

// method based on https://github.com/zhimoe/mdict-rs/blob/master/src/query/mod.rs (unknown license but very generic code)
pub fn query_internal(word: String, path_db: PathBuf) -> String {
    let w = word;

    let conn = Connection::open(&path_db).unwrap();
    let mut stmt = conn
        .prepare("select * from MDX_INDEX WHERE text= :word limit 1;")
        .unwrap();

    let mut rows = stmt.query(named_params! { ":word": w }).unwrap();
    let row = rows.next().unwrap();
    if let Some(row) = row {
        let def = row.get::<usize, String>(1).unwrap();
        return def;
    }

    "not found".to_string()
}

#[tauri::command]
fn get_content(handle: tauri::AppHandle) -> String {
    let resource_path = handle
        .path_resolver()
        .resolve_resource("src/sakubi.html")
        .expect("failed to resolve resource.");

    if let Ok(result) = fs::read_to_string(resource_path) {
        return result;
    }
    "Failed to read content".to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_content, definition])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
