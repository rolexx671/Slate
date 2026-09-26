//! Split into new files without changing the open source document.
use serde::Serialize;
use lopdf::{Document, Object, ObjectId};
use std::collections::HashSet;
use sofdocs_desktop::pdf_ops;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize)]
pub struct SplitResult {
    pub paths: Vec<String>,
    pub directory: Option<String>,
}

pub fn validate_groups(bytes: &[u8], groups: &[Vec<u32>]) -> Result<(), String> {
    let count = pdf_ops::page_count(bytes.to_vec())?;
    if groups.is_empty() || groups.len() > 10000 || groups.iter().map(Vec::len).sum::<usize>() > 100000 {
        return Err("Укажите от 1 до 10 000 выходных PDF-файлов.".into());
    }
    for group in groups {
        if group.is_empty() || group.iter().any(|&n| n == 0 || n > count) {
            return Err(format!("Номера страниц должны быть от 1 до {count}."));
        }
    }
    Ok(())
}

fn stem(filename: &str) -> String {
    let value = Path::new(filename).file_stem().and_then(|v| v.to_str()).unwrap_or("Документ");
    let safe: String = value.chars().filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c)).take(64).collect();
    let safe = safe.trim().trim_matches('.');
    if safe.is_empty() { "Документ".into() } else { safe.into() }
}

pub fn suggested_name(filename: &str, pages: &[u32]) -> String {
    let mut pages = pages.to_vec(); pages.sort_unstable(); pages.dedup();
    let label = match pages.as_slice() {
        [] => "выбранные".into(),
        [n] => n.to_string(),
        p if p.windows(2).all(|w| w[1] == w[0] + 1) => format!("{}-{}", p[0], p[p.len()-1]),
        p if p.len() <= 8 => p.iter().map(u32::to_string).collect::<Vec<_>>().join("_"),
        _ => "выбранные".into(),
    };
    format!("{}_стр_{label}.pdf", stem(filename))
}

// Keep only fields belonging to retained pages. A plain page deletion otherwise
// leaves invisible values from removed pages in the document-wide AcroForm tree.
fn keep_field(doc: &mut Document, id: ObjectId, widgets: &HashSet<ObjectId>, kept: &mut HashSet<ObjectId>, visiting: &mut HashSet<ObjectId>) -> bool {
    if !visiting.insert(id) { return false; }
    let children = doc.get_dictionary(id).ok().and_then(|d| d.get(b"Kids").ok())
        .and_then(|v| v.as_array().ok()).cloned();
    let keep = if let Some(children) = children {
        let children: Vec<Object> = children.into_iter().filter(|child| child.as_reference().ok()
            .is_some_and(|child| keep_field(doc, child, widgets, kept, visiting))).collect();
        let keep = !children.is_empty();
        if let Ok(dict) = doc.get_dictionary_mut(id) { dict.set("Kids", children); }
        keep
    } else { widgets.contains(&id) };
    visiting.remove(&id);
    if keep { kept.insert(id); }
    keep
}

pub fn extract_selected(bytes: &[u8], pages: &[u32]) -> Result<Vec<u8>, String> {
    let output = pdf_ops::extract_pages(bytes.to_vec(), pages.to_vec())?;
    let mut doc = Document::load_mem(&output).map_err(|e| e.to_string())?;
    let form = doc.catalog().ok().and_then(|d| d.get(b"AcroForm").ok()).cloned();
    let Some(form) = form else { return Ok(output); };
    let mut form_dict = match &form {
        Object::Reference(id) => doc.get_dictionary(*id).map_err(|e| e.to_string())?.clone(),
        Object::Dictionary(dict) => dict.clone(),
        _ => return Ok(output),
    };
    let mut widgets = HashSet::new();
    for id in doc.get_pages().values() {
        if let Ok(annotations) = doc.get_dictionary(*id).and_then(|d| d.get(b"Annots")).and_then(Object::as_array) {
            for item in annotations { if let Ok(id) = item.as_reference() { widgets.insert(id); } }
        }
    }
    let roots = form_dict.get(b"Fields").and_then(Object::as_array).cloned().unwrap_or_default();
    let mut kept = HashSet::new();
    let roots: Vec<Object> = roots.into_iter().filter(|root| root.as_reference().ok()
        .is_some_and(|id| keep_field(&mut doc, id, &widgets, &mut kept, &mut HashSet::new()))).collect();
    form_dict.set("Fields", roots);
    if let Ok(order) = form_dict.get(b"CO").and_then(Object::as_array) {
        let order: Vec<Object> = order.iter().filter(|v| v.as_reference().ok().is_some_and(|id| kept.contains(&id))).cloned().collect();
        form_dict.set("CO", order);
    }
    match form {
        Object::Reference(id) => { doc.objects.insert(id, Object::Dictionary(form_dict)); }
        _ => { let id = doc.trailer.get(b"Root").and_then(Object::as_reference).map_err(|e| e.to_string())?;
            doc.get_dictionary_mut(id).map_err(|e| e.to_string())?.set("AcroForm", form_dict); }
    }
    doc.prune_objects();
    let mut cleaned = Vec::new(); doc.save_to(&mut cleaned).map_err(|e| e.to_string())?;
    Ok(cleaned)
}

fn same_file(a: &Path, b: &Path) -> bool {
    if a == b { return true; }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

pub fn write_single(bytes: &[u8], pages: &[u32], destination: &Path, source: Option<&Path>) -> Result<(), String> {
    validate_groups(bytes, &[pages.to_vec()])?;
    if source.is_some_and(|p| same_file(p, destination)) {
        return Err("Выберите другое имя или папку: исходный PDF нельзя перезаписать при разделении.".into());
    }
    let output = extract_selected(bytes, pages)?;
    let parent = destination.parent().ok_or("Не удалось определить папку сохранения.")?;
    let mut temp = None;
    for attempt in 0..1000 {
        let candidate = parent.join(format!(".slate-split-{}-{attempt}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => { temp = Some((candidate, file)); break; }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Не удалось создать файл: {e}")),
        }
    }
    let (temp_path, mut file) = temp.ok_or("Не удалось создать временный файл.")?;
    let result = file.write_all(&output).and_then(|_| file.sync_all()).and_then(|_| fs::rename(&temp_path, destination));
    if let Err(error) = result {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Не удалось сохранить PDF: {error}"));
    }
    Ok(())
}

pub fn write_multiple(bytes: &[u8], groups: &[Vec<u32>], parent: &Path, filename: &str) -> Result<SplitResult, String> {
    validate_groups(bytes, groups)?;
    let base = format!("{} — страницы", stem(filename));
    let mut chosen = None;
    for n in 1..10000 {
        let folder = parent.join(if n == 1 { base.clone() } else { format!("{base} ({n})") });
        match fs::create_dir(&folder) {
            Ok(()) => { chosen = Some(folder); break; }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Не удалось создать папку: {e}")),
        }
    }
    let folder = chosen.ok_or("Не удалось выбрать свободное имя папки.")?;
    let mut paths = Vec::new();
    for (index, pages) in groups.iter().enumerate() {
        let target = folder.join(format!("{:03}_{}", index+1, suggested_name(filename, pages)));
        if let Err(error) = write_single(bytes, pages, &target, None) {
            // Only remove files produced by this invocation, never existing user files.
            for path in &paths { let _ = fs::remove_file(path); }
            let _ = fs::remove_dir(&folder);
            return Err(format!("Разделение не завершено: {error}. Папка результата: {}", folder.display()));
        }
        paths.push(target.to_string_lossy().into_owned());
    }
    Ok(SplitResult { paths, directory: Some(folder.to_string_lossy().into_owned()) })
}

#[tauri::command]
pub async fn split_pdf_dialog(app: AppHandle, bytes: Vec<u8>, groups: Vec<Vec<u32>>, filename: String, source_path: Option<String>) -> Result<Option<SplitResult>, String> {
    let (bytes, groups) = tauri::async_runtime::spawn_blocking(move || {
        validate_groups(&bytes, &groups)?;
        Ok::<_, String>((bytes, groups))
    }).await.map_err(|e| format!("Не удалось подготовить разделение: {e}"))??;
    if groups.len() == 1 {
        let selected = app.dialog().file().set_title("Сохранить выбранные страницы")
            .add_filter("Документ PDF", &["pdf"]).set_file_name(suggested_name(&filename, &groups[0])).blocking_save_file();
        let Some(selected) = selected else { return Ok(None); };
        let mut destination = selected.into_path().map_err(|e| e.to_string())?;
        if destination.extension().is_none() { destination.set_extension("pdf"); }
        let source = source_path.map(PathBuf::from);
        tauri::async_runtime::spawn_blocking(move || {
            write_single(&bytes, &groups[0], &destination, source.as_deref())?;
            Ok(Some(SplitResult { paths: vec![destination.to_string_lossy().into_owned()], directory: None }))
        }).await.map_err(|e| format!("Не удалось разделить PDF: {e}"))?
    } else {
        let selected = app.dialog().file().set_title("Выберите папку для разделённого PDF").blocking_pick_folder();
        let Some(selected) = selected else { return Ok(None); };
        let parent = selected.into_path().map_err(|e| e.to_string())?;
        tauri::async_runtime::spawn_blocking(move || write_multiple(&bytes, &groups, &parent, &filename).map(Some))
            .await.map_err(|e| format!("Не удалось разделить PDF: {e}"))?
    }
}
