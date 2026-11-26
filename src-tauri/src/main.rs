// Windows 릴리스 빌드에서 콘솔 창이 뜨지 않게 한다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

#[derive(Serialize)]
struct Note {
    id: String,
    text: String,
    mtime: f64,
}

/// 메모 저장 폴더: 문서/메모장 (없으면 만든다)
fn dir() -> Result<PathBuf, String> {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or("문서 폴더를 찾을 수 없습니다")?;
    let d = base.join("메모장");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

/// 파일 이름으로 쓸 id는 영숫자만 허용 (경로 조작 방지)
fn file_of(id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("잘못된 메모 id".into());
    }
    Ok(dir()?.join(format!("{id}.txt")))
}

#[tauri::command]
fn notes_dir() -> Result<String, String> {
    Ok(dir()?.display().to_string())
}

#[tauri::command]
fn list_notes() -> Result<Vec<Note>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir()?).map_err(|e| e.to_string())? {
        let path = match entry {
            Ok(e) => e.path(),
            Err(_) => continue,
        };
        if path.extension().and_then(|x| x.to_str()) != Some("txt") {
            continue;
        }
        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        // 직접 넣은 .txt 파일(한글 이름 등)도 읽되, 앱이 다시 쓸 수 있는 이름만 목록에 올린다
        if !id.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        let mtime = fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);
        out.push(Note { id, text, mtime });
    }
    Ok(out)
}

#[tauri::command]
fn save_note(id: String, text: String) -> Result<(), String> {
    fs::write(file_of(&id)?, text).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_note(id: String) -> Result<(), String> {
    let f = file_of(&id)?;
    if !f.exists() {
        return Ok(());
    }
    trash::delete(&f).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_folder() -> Result<(), String> {
    let d = dir()?;
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let cmd = "xdg-open";
    std::process::Command::new(cmd)
        .arg(d)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            notes_dir,
            list_notes,
            save_note,
            delete_note,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("메모장을 시작하지 못했습니다");
}
