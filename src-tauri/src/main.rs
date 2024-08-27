// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use magnum::container::ogg::OpusSourceOgg;

use rusqlite::{named_params, Connection};
use std::fs::File;
use std::num::ParseIntError;
use std::{fs, io::Read, path::PathBuf};

use tauri::{CustomMenuItem, Menu, Submenu};

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

// db generated same as for definition.
#[tauri::command]
fn play_audio(handle: tauri::AppHandle, word: String) -> String {
    let db_path = handle
        .path_resolver()
        .resolve_resource("src/jitendex.audio.db")
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path_resolver()
        .resolve_resource("src/jitindex.audio.dict")
        .expect("failed to resolve resource.");
    let mut dictionary = Vec::new();
    let mut f = File::open(path_dictionary.clone()).unwrap();
    let _ = f.read_to_end(&mut dictionary);

    let conn = Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT hex(data)
            FROM audio
            where id = :word;",
        )
        .unwrap();
    let mut row = stmt.query(named_params! {":word": word}).unwrap();
    if let Some(row) = row.next().unwrap() {
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        let hexstr = row.get::<usize, String>(0).unwrap();

        let compressed = decode_hex(&hexstr).unwrap();
        let reader = std::io::BufReader::new(compressed.as_slice());
        let mut decoder = zstd::Decoder::with_dictionary(reader, &dictionary).unwrap();
        let mut buf = Vec::new();
        let _ = decoder.read_to_end(&mut buf);

        let buf_reader = std::io::BufReader::new(std::io::Cursor::new(buf));
        let source = OpusSourceOgg::new(buf_reader).unwrap();
        // using sink to sleep to end.
        sink.append(source);
        sink.sleep_until_end();
    }
    "".to_string()
}

// definition comes from https://jitendex.org/pages/downloads.html ( https://creativecommons.org/licenses/by-sa/4.0/)
// MDict db and zstd dictionary has been generated using https://github.com/kentaromiura/jitendex-analysis (GPL 3.0)
// data is CC 4.0 as per above.
#[tauri::command]
fn definition(handle: tauri::AppHandle, word: String) -> String {
    let db_path = handle
        .path_resolver()
        .resolve_resource("src/jitendex.comp.db")
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path_resolver()
        .resolve_resource("src/jitindex.dict")
        .expect("failed to resolve resource.");

    let def = query_internal(
        word.trim().to_string(),
        db_path.clone(),
        path_dictionary.clone(),
    );

    return def;
}

pub fn query_internal(word: String, path_db: PathBuf, path_dictionary: PathBuf) -> String {
    let w = word;
    let mut dictionary = Vec::new();
    let mut f = File::open(path_dictionary.clone()).unwrap();
    let _ = f.read_to_end(&mut dictionary);
    let conn = Connection::open(&path_db).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT hex([d].definition)
            FROM terms
            LEFT JOIN definitions d
            ON d.id = [terms].definition
            WHERE term = :word;",
        )
        .unwrap();

    let mut rows = stmt.query(named_params! { ":word": w }).unwrap();
    let mut result = String::new();
    while let Some(row) = rows.next().unwrap() {
        let hexstr = row.get::<usize, String>(0).unwrap();
        let mut buf = Vec::new();
        let def = decode_hex(&hexstr).unwrap();
        let reader = std::io::BufReader::new(def.as_slice());
        let mut decoder = zstd::Decoder::with_dictionary(reader, &dictionary).unwrap();
        let _ = decoder.read_to_end(&mut buf);
        result.push_str(&String::from_utf8(buf).unwrap());
    }
    if result.len() > 0 {
        return result;
    }

    "not found".to_string()
}

#[tauri::command]
fn query_by_id(handle: tauri::AppHandle, id: String) -> String {
    let db_path = handle
        .path_resolver()
        .resolve_resource("src/jitendex.comp.db")
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path_resolver()
        .resolve_resource("src/jitindex.dict")
        .expect("failed to resolve resource.");
    let mut dictionary = Vec::new();
    let mut f = File::open(path_dictionary.clone()).unwrap();
    let _ = f.read_to_end(&mut dictionary);
    let conn = Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT hex([d].definition)
            FROM terms
            LEFT JOIN definitions d
            ON d.id = [terms].definition
            WHERE d.id = :id",
        )
        .unwrap();
    let mut rows = stmt.query(named_params! {":id": id}).unwrap();
    let mut result = String::new();
    while let Some(row) = rows.next().unwrap() {
        let hexstr = row.get::<usize, String>(0).unwrap();
        let mut buf = Vec::new();
        let def = decode_hex(&hexstr).unwrap();
        let reader = std::io::BufReader::new(def.as_slice());
        let mut decoder = zstd::Decoder::with_dictionary(reader, &dictionary).unwrap();
        let _ = decoder.read_to_end(&mut buf);
        result.push_str(&String::from_utf8(buf).unwrap());
    }
    if result.len() > 0 {
        return result;
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
    let submenu = Submenu::new(
        "Actions",
        Menu::new().add_item(CustomMenuItem::new("lookup", "lookup")),
    );
    let menu = Menu::os_default("Sakubi Reader").add_submenu(submenu);

    tauri::Builder::default()
        .menu(menu)
        .on_menu_event(|event| match event.menu_item_id() {
            "lookup" => {
                let _ = event.window().emit("lookup", ()).unwrap();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_content,
            definition,
            query_by_id,
            play_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
