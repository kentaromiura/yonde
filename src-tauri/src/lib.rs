use magnum::container::ogg::OpusSourceOgg;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu},
    Emitter, Manager, State,
};

use rusqlite::{named_params, Connection};
use std::fs::File;
use std::io::Cursor;
use std::num::ParseIntError;
use std::{fs, io::Read, path::PathBuf};

use cbz::CbzArchive;

struct AppState {
    archives: Mutex<HashMap<String, CbzArchive<Cursor<Vec<u8>>>>>,
    ocr: Arc<Mutex<Option<comic_ocr::manga_ocr::MangaOcr>>>,
    detector: Arc<Mutex<Option<comic_ocr::comic_text_detector::ComicTextDetector>>>,
}

const MENU_EVENT_LOOKUP: &str = "lookup";

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
        .path()
        .resolve(
            "src/jitendex.audio.db",
            tauri::path::BaseDirectory::Resource,
        )
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path()
        .resolve(
            "src/jitindex.audio.dict",
            tauri::path::BaseDirectory::Resource,
        )
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
        .path()
        .resolve("src/jitendex.comp.db", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path()
        .resolve("src/jitindex.dict", tauri::path::BaseDirectory::Resource)
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
        .path()
        .resolve("src/jitendex.comp.db", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve resource.");
    let path_dictionary = handle
        .path()
        .resolve("src/jitindex.dict", tauri::path::BaseDirectory::Resource)
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
        .path()
        .resolve("src/sakubi.html", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve resource.");

    if let Ok(result) = fs::read_to_string(resource_path) {
        return result;
    }
    "Failed to read content".to_string()
}

#[tauri::command]
async fn open_cbz(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<String>, String> {
    println!("[Rust] open_cbz called with path: {}", path);
    let data = tokio::fs::read(&path).await.map_err(|e| {
        let err = format!("Failed to read file: {}", e);
        println!("[Rust] Error: {}", err);
        err
    })?;
    println!("[Rust] Read {} bytes", data.len());
    
    let archive = CbzArchive::from_bytes(data).map_err(|e| {
        let err = format!("Failed to parse CBZ: {}", e);
        println!("[Rust] Error: {}", err);
        err
    })?;
    let pages = archive.image_names();
    println!("[Rust] Found {} pages", pages.len());
    
    let id = path.clone();
    
    let mut archives = state.archives.lock().unwrap();
    archives.insert(id, archive);
    
    Ok(pages)
}

#[tauri::command]
fn close_cbz(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let mut archives = state.archives.lock().unwrap();
    archives.remove(&path);
    Ok(())
}

#[derive(serde::Serialize)]
struct PageResult {
    image: String,
    mime_type: String,
    width: u32,
    height: u32,
}

#[tauri::command]
fn get_page(
    state: State<'_, AppState>,
    path: String,
    page_name: String,
) -> Result<PageResult, String> {
    println!("[Rust] get_page called: {} / {}", path, page_name);
    let mut archives = state.archives.lock().unwrap();
    let archive = archives.get_mut(&path).ok_or("Archive not opened")?;
    
    let image = archive.read_image(&page_name).map_err(|e| e.to_string())?;
    let mime_type = image.mime_type().to_string();
    println!("[Rust] Image mime_type: {}, size: {} bytes", mime_type, image.data.len());
    
    let img = image::load_from_memory(&image.data).map_err(|e| e.to_string())?;
    let (width, height) = (img.width(), img.height());
    println!("[Rust] Image dimensions: {}x{}", width, height);
    
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image.data);
    
    Ok(PageResult {
        image: encoded,
        mime_type,
        width,
        height,
    })
}

#[derive(serde::Serialize)]
struct OcrResult {
    text: String,
    bbox: (usize, usize, usize, usize),
    confidence: f32,
}

#[derive(serde::Serialize)]
struct PageWithOcrResult {
    image: String,
    mime_type: String,
    width: u32,
    height: u32,
    ocr_results: Vec<OcrResult>,
}

#[tauri::command]
async fn get_page_with_ocr(
    state: State<'_, AppState>,
    path: String,
    page_name: String,
) -> Result<PageWithOcrResult, String> {
    println!("[Rust] get_page_with_ocr called: {} / {}", path, page_name);
    let image_data = {
        let mut archives = state.archives.lock().unwrap();
        let archive = archives.get_mut(&path).ok_or("Archive not opened")?;
        archive.read_image(&page_name).map_err(|e| e.to_string())?
    };
    
    let mime_type = image_data.mime_type().to_string();
    let img = image::load_from_memory(&image_data.data).map_err(|e| e.to_string())?;
    let (width, height) = (img.width(), img.height());
    println!("[Rust] Image: {}x{}", width, height);
    
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data.data);
    
    // Clone Arc references for the thread
    let detector_arc = state.detector.clone();
    let ocr_arc = state.ocr.clone();
    let img_bytes = image_data.data.clone();
    
    // Check if OCR is initialized
    {
        let det_guard = detector_arc.lock().unwrap();
        let ocr_guard = ocr_arc.lock().unwrap();
        if det_guard.is_none() || ocr_guard.is_none() {
            println!("[Rust] OCR not initialized, returning empty results");
            return Ok(PageWithOcrResult {
                image: encoded,
                mime_type,
                width,
                height,
                ocr_results: Vec::new(),
            });
        }
    }
    
    // Run OCR in a thread with larger stack
    let ocr_results = std::thread::Builder::new()
        .stack_size(4 * 1024 * 1024) // 4MB stack for inference
        .spawn(move || {
            let mut img = image::load_from_memory(&img_bytes).unwrap();
            
            let det_guard = detector_arc.lock().unwrap();
            let ocr_guard = ocr_arc.lock().unwrap();
            
            if let (Some(detector), Some(ocr)) = (det_guard.as_ref(), ocr_guard.as_ref()) {
                println!("[Rust] Running text detection...");
                match detector.inference(&img) {
                    Ok(bboxes) => {
                        println!("[Rust] Found {} text regions", bboxes.len());
                        let mut results = Vec::new();
                        for bbox in bboxes {
                            let crop = img.crop(
                                bbox.xmin as u32,
                                bbox.ymin as u32,
                                (bbox.xmax - bbox.xmin) as u32,
                                (bbox.ymax - bbox.ymin) as u32,
                            );
                            
                            if let Ok(texts) = ocr.inference(&[crop]) {
                                if let Some(text) = texts.first() {
                                    if !text.is_empty() {
                                        println!("[Rust] OCR text: {} (confidence: {:.2})", text, bbox.confidence);
                                        results.push(OcrResult {
                                            text: text.clone(),
                                            bbox: (bbox.xmin, bbox.ymin, bbox.xmax, bbox.ymax),
                                            confidence: bbox.confidence,
                                        });
                                    }
                                }
                            }
                        }
                        results
                    }
                    Err(e) => {
                        println!("[Rust] Detection error: {}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        })
        .map_err(|e| format!("Failed to spawn thread: {}", e))?
        .join()
        .map_err(|_| "Thread panicked".to_string())?;
    
    println!("[Rust] Returning {} OCR results", ocr_results.len());
    Ok(PageWithOcrResult {
        image: encoded,
        mime_type,
        width,
        height,
        ocr_results,
    })
}

#[tauri::command]
async fn init_ocr(state: State<'_, AppState>) -> Result<(), String> {
    println!("[Rust] init_ocr called");
    {
        let detector = state.detector.lock().unwrap();
        if detector.is_some() {
            println!("[Rust] OCR already initialized");
            return Ok(());
        }
    }
    
    let detector = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8MB stack
        .spawn(|| {
            println!("[Rust] Loading comic text detector...");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                comic_ocr::comic_text_detector::ComicTextDetector::load(false).await
            })
        })
        .map_err(|e| format!("Failed to spawn thread: {}", e))?
        .join()
        .map_err(|_| "Thread panicked".to_string())?
        .map_err(|e| {
            let err = format!("Failed to load detector: {}", e);
            println!("[Rust] Error: {}", err);
            err
        })?;
    println!("[Rust] Detector loaded");
    
    let ocr = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8MB stack
        .spawn(|| {
            println!("[Rust] Loading manga OCR...");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                comic_ocr::manga_ocr::MangaOcr::load(false).await
            })
        })
        .map_err(|e| format!("Failed to spawn thread: {}", e))?
        .join()
        .map_err(|_| "Thread panicked".to_string())?
        .map_err(|e| {
            let err = format!("Failed to load OCR: {}", e);
            println!("[Rust] Error: {}", err);
            err
        })?;
    println!("[Rust] OCR loaded");
    
    {
        let mut det_guard = state.detector.lock().unwrap();
        *det_guard = Some(detector);
    }
    
    {
        let mut ocr_guard = state.ocr.lock().unwrap();
        *ocr_guard = Some(ocr);
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let detector = Arc::new(Mutex::new(None));
    let ocr = Arc::new(Mutex::new(None));
    
    let state = AppState {
        archives: Mutex::new(HashMap::new()),
        ocr: ocr.clone(),
        detector: detector.clone(),
    };
    
    tauri::Builder::default()
        .manage(state)
        .menu(|handle| {
            Menu::with_items(
                handle,
                &[&Submenu::with_items(
                    handle,
                    "",
                    true,
                    &[
                        &PredefinedMenuItem::close_window(handle, None)?,
                        #[cfg(target_os = "macos")]
                        &MenuItemBuilder::new(MENU_EVENT_LOOKUP).id("lookup").build(handle)?,
                    ],
                )?],
            )
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_EVENT_LOOKUP=> {
                let _ = app.emit(MENU_EVENT_LOOKUP, ());
            }
            _ => {
                println!("no matches. {}", event.id().as_ref())
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_content,
            definition,
            query_by_id,
            play_audio,
            open_cbz,
            close_cbz,
            get_page,
            get_page_with_ocr,
            init_ocr
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
