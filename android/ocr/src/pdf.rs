//! The PDF reader of Attachment Texts on the phones (#267, ADR 0047):
//! the desktop's `oldbrainfix-pdf`, built into this library beside
//! RustO, so a PDF reads the same on the phone and on the computer. The
//! app hands in the path of a copy of the PDF in its cache; each call
//! reads one page, so at most one page's scan is in memory at a time.
//!
//! A page goes to the app as one byte array: the length of its text in
//! UTF-8 as four bytes, most significant first, the text, then the
//! picture of a scanned page as it lies in the PDF (JPEG) or as a PNG,
//! nothing when it is no scan ([encode_page]).

use std::ptr;

use jni::objects::{JObject, JString};
use jni::sys::{jbyteArray, jint};
use jni::JNIEnv;
use oldbrainfix_pdf::{Page, Pdf};

/// How many pages the PDF `bytes` has; none when it reads as no PDF.
/// An encrypted PDF still counts its pages.
pub fn page_count(bytes: &[u8]) -> Option<u64> {
    oldbrainfix_pdf::page_count(bytes)
}

/// The page `index` of the PDF `bytes`, counted from 0; none when there
/// is no such page or the PDF cannot be read, as an encrypted one.
pub fn page(bytes: &[u8], index: usize) -> Option<Page> {
    Pdf::open(bytes)?.page(index)
}

/// `page` as the app reads it: the text's length in four bytes, most
/// significant first, the text in UTF-8, then the scan's bytes.
pub fn encode_page(page: &Page) -> Vec<u8> {
    let text = page.text.as_bytes();
    // A page's text is far below 4 GiB: its content is at most 64 MiB.
    let len = u32::try_from(text.len()).unwrap_or(u32::MAX);
    let kept = text.get(..len as usize).unwrap_or(text);
    let scan = page.scan.as_deref().unwrap_or(&[]);
    let mut out = Vec::with_capacity(4 + kept.len() + scan.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(kept);
    out.extend_from_slice(scan);
    out
}

/// The pages of the PDF at `path` as [page_count] counts them, -1 when
/// it cannot be read or is no PDF.
pub fn pages_at(path: &str) -> i64 {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| page_count(&bytes))
        .and_then(|n| i64::try_from(n).ok())
        .unwrap_or(-1)
}

/// The page `index` of the PDF at `path`, encoded ([encode_page]).
pub fn page_at(path: &str, index: usize) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    page(&bytes, index).map(|p| encode_page(&p))
}

/// `PdfNative.pages(path)`: the PDF's pages, -1 when it cannot be read.
/// A panic never crosses into Java.
// SAFETY: the one exported symbol of this name, with the signature
// PdfNative's `external fun pages` declares.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_PdfNative_pages<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    path: JString<'local>,
) -> jint {
    let path: Option<String> = env.get_string(&path).ok().map(Into::into);
    let pages = std::panic::catch_unwind(|| path.map_or(-1, |p| pages_at(&p)));
    match pages {
        Ok(n) => jint::try_from(n).unwrap_or(jint::MAX),
        Err(_) => -1,
    }
}

/// `PdfNative.page(path, index)`: the page encoded ([encode_page]), or
/// null when it cannot be read.
// SAFETY: the export as in `pages`.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_PdfNative_page<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    path: JString<'local>,
    index: jint,
) -> jbyteArray {
    let path: Option<String> = env.get_string(&path).ok().map(Into::into);
    let page = std::panic::catch_unwind(|| {
        let index = usize::try_from(index).ok()?;
        page_at(&path?, index)
    });
    match page {
        Ok(Some(bytes)) => env
            .byte_array_from_slice(&bytes)
            .map(|array| array.into_raw())
            .unwrap_or(ptr::null_mut()),
        _ => ptr::null_mut(),
    }
}

/// The C functions the iPhone's `FilesChannel.swift` calls, the twins
/// of the JNI ones above, declared in `ios/Runner/oldbrainfix_native.h`.
/// Built for the host too in tests, so they run here.
// SAFETY: FFI; each block says why it holds.
#[allow(unsafe_code)]
#[cfg(any(target_os = "ios", test))]
pub mod ios {
    use std::ffi::{c_char, CStr};
    use std::panic::catch_unwind;
    use std::ptr;

    /// The path at `path` as UTF-8; none for null or other bytes.
    ///
    /// # Safety
    ///
    /// A non-null `path` points to a NUL-terminated string that stays
    /// unchanged for the length of the call.
    unsafe fn path_of(path: *const c_char) -> Option<String> {
        if path.is_null() {
            return None;
        }
        // SAFETY: the caller's promise above.
        let path = unsafe { CStr::from_ptr(path) };
        path.to_str().ok().map(String::from)
    }

    /// The pages of the PDF at `path`, -1 when it cannot be read.
    ///
    /// # Safety
    ///
    /// As [path_of].
    #[no_mangle]
    pub unsafe extern "C" fn obf_pdf_pages(path: *const c_char) -> i64 {
        // SAFETY: the caller's promise above.
        let path = unsafe { path_of(path) };
        catch_unwind(|| path.map_or(-1, |p| super::pages_at(&p))).unwrap_or(-1)
    }

    /// The page `index` of the PDF at `path`, encoded as for Android, its
    /// length written to `len`; null when it cannot be read. Only
    /// [obf_pdf_free] frees it.
    ///
    /// # Safety
    ///
    /// As [path_of]; `len` is null or points to a writable `usize`.
    #[no_mangle]
    pub unsafe extern "C" fn obf_pdf_page(
        path: *const c_char,
        index: i64,
        len: *mut usize,
    ) -> *mut u8 {
        if len.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: the caller's promise above.
        let path = unsafe { path_of(path) };
        let page = catch_unwind(|| {
            let index = usize::try_from(index).ok()?;
            super::page_at(&path?, index)
        });
        let Ok(Some(bytes)) = page else {
            return ptr::null_mut();
        };
        let boxed = bytes.into_boxed_slice();
        // SAFETY: `len` is writable, the caller's promise above.
        unsafe { *len = boxed.len() };
        Box::into_raw(boxed).cast::<u8>()
    }

    /// Frees a page [obf_pdf_page] gave, of `len` bytes; null is fine.
    ///
    /// # Safety
    ///
    /// `page` is null or came from [obf_pdf_page] with this `len`, and is
    /// not used again.
    #[no_mangle]
    pub unsafe extern "C" fn obf_pdf_free(page: *mut u8, len: usize) {
        if page.is_null() {
            return;
        }
        let _ = catch_unwind(|| {
            // SAFETY: the caller's promise above: the boxed slice it was.
            drop(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(page, len)) })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// A PDF of `pages` pages, page n showing "Page n" in Helvetica.
    fn pdf(pages: usize) -> Vec<u8> {
        let mut objects: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            String::new(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
        ];
        let mut kids = Vec::new();
        for n in 1..=pages {
            let content = format!("BT /F1 12 Tf 72 720 Td (Page {n}) Tj ET");
            objects.push(format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ));
            let content_id = objects.len();
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            ));
            kids.push(format!("{} 0 R", objects.len()));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            kids.join(" ")
        );
        let mut out = String::from("%PDF-1.4\n");
        for (i, body) in objects.iter().enumerate() {
            out.push_str(&format!("{} 0 obj\n{body}\nendobj\n", i + 1));
        }
        out.push_str(&format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\n%%EOF\n",
            objects.len() + 1
        ));
        out.into_bytes()
    }

    fn decode(bytes: &[u8]) -> (String, Vec<u8>) {
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        (
            String::from_utf8(bytes[4..4 + len].to_vec()).unwrap(),
            bytes[4 + len..].to_vec(),
        )
    }

    #[test]
    fn counts_and_reads_each_page() {
        let bytes = pdf(3);
        assert_eq!(page_count(&bytes), Some(3));
        assert_eq!(page(&bytes, 1).unwrap().text, "Page 2");
        assert!(page(&bytes, 3).is_none());
        assert_eq!(page_count(b"no pdf at all"), None);
    }

    #[test]
    fn a_page_is_its_text_length_its_text_and_its_scan() {
        let plain = Page {
            text: "Grüße".into(),
            scan: None,
        };
        let encoded = encode_page(&plain);
        assert_eq!(&encoded[..4], &[0, 0, 0, 7]);
        assert_eq!(decode(&encoded), ("Grüße".to_string(), vec![]));
        let scan = Page {
            text: String::new(),
            scan: Some(vec![0xFF, 0xD8, 0xFF]),
        };
        assert_eq!(
            decode(&encode_page(&scan)),
            (String::new(), vec![0xFF, 0xD8, 0xFF])
        );
    }

    // SAFETY: the C functions are called with valid strings and lengths.
    #[allow(unsafe_code)]
    #[test]
    fn reads_from_a_path_and_says_minus_one_without_a_pdf() {
        let dir = std::env::temp_dir().join(format!("obf-pdf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("letter.pdf");
        std::fs::write(&file, pdf(2)).unwrap();
        let path = file.to_str().unwrap();
        assert_eq!(pages_at(path), 2);
        assert_eq!(decode(&page_at(path, 0).unwrap()).0, "Page 1");
        assert_eq!(pages_at(dir.join("gone.pdf").to_str().unwrap()), -1);
        assert!(page_at(path, 2).is_none());

        // The iPhone's twins.
        let c_path = CString::new(path).unwrap();
        unsafe {
            assert_eq!(ios::obf_pdf_pages(c_path.as_ptr()), 2);
            assert_eq!(ios::obf_pdf_pages(std::ptr::null()), -1);
            let mut len = 0usize;
            let page = ios::obf_pdf_page(c_path.as_ptr(), 1, &mut len);
            assert!(!page.is_null());
            let bytes = std::slice::from_raw_parts(page, len).to_vec();
            ios::obf_pdf_free(page, len);
            assert_eq!(decode(&bytes).0, "Page 2");
            assert!(ios::obf_pdf_page(c_path.as_ptr(), 9, &mut len).is_null());
            assert!(ios::obf_pdf_page(c_path.as_ptr(), 0, std::ptr::null_mut()).is_null());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
