//! In-memory ZIP generation (pure Rust) + browser download trigger via web-sys.

use indexmap::IndexMap;
use std::io::{Cursor, Write};
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};
use zip::{write::FileOptions, ZipWriter};

/// Build a ZIP archive in memory from a `path -> content` map.
pub fn build_zip(files: &IndexMap<String, String>) -> Result<Vec<u8>, String> {
    let buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buf);

    let opts: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (path, content) in files {
        zip.start_file(path.as_str(), opts)
            .map_err(|e| format!("ZIP start_file error: {e}"))?;
        zip.write_all(content.as_bytes()).map_err(|e| format!("ZIP write error: {e}"))?;
    }

    let inner = zip.finish().map_err(|e| format!("ZIP finish error: {e}"))?;
    Ok(inner.into_inner())
}

/// Trigger a browser file-download of `bytes` with the given `filename`.
pub fn trigger_download(bytes: Vec<u8>, filename: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    // Build a Blob from the bytes
    let js_arr = js_sys::Uint8Array::from(bytes.as_slice());
    let array = js_sys::Array::new();
    array.push(&js_arr);

    let bag = BlobPropertyBag::new();
    bag.set_type("application/zip");

    let blob = Blob::new_with_u8_array_sequence_and_options(&array, &bag)
        .map_err(|e| format!("Blob error: {e:?}"))?;

    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("createObjectURL error: {e:?}"))?;

    // Create a temporary hidden <a> element, click it, then remove it.
    let a: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("createElement error: {e:?}"))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| "Cast to HtmlAnchorElement failed")?;

    a.set_href(&url);
    a.set_download(filename);
    a.set_attribute("style", "display:none")
        .map_err(|e| format!("setAttribute error: {e:?}"))?;

    let body = document.body().ok_or("No body")?;
    body.append_child(&a).map_err(|e| format!("appendChild error: {e:?}"))?;

    a.click();

    // Clean up: remove via parent node
    let _ = body.remove_child(&a);
    let _ = Url::revoke_object_url(&url);

    Ok(())
}
