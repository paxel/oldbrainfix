//! Text in photos for Old Brain Fix on Android (ADR 0007): ocrs, the
//! engine Catlog's desktop uses, with its detection and recognition
//! models bundled in the app, behind three JNI functions for
//! `OcrNative` in the app's Kotlin. Fully offline, no Google library.
//!
//! The app decodes the photo itself (an upright JPEG of at most 2048
//! px) and hands in its RGBA pixels, so no image decoder is built in.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use jni::objects::{JByteArray, JObject};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

/// ocrs with both models loaded, ready to read photos.
pub struct Reader {
    engine: OcrEngine,
}

impl Reader {
    /// Loads the text-detection and text-recognition models from the
    /// bytes of their `.rten` files.
    pub fn new(detection: Vec<u8>, recognition: Vec<u8>) -> Result<Reader, String> {
        let detection = Model::load(detection).map_err(|e| e.to_string())?;
        let recognition = Model::load(recognition).map_err(|e| e.to_string())?;
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
        Ok(Reader { engine })
    }

    /// The text in a picture of `width` × `height` RGBA pixels, row by
    /// row: one line per line of text, top to bottom; empty when there
    /// is none.
    pub fn read(&self, rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
        check_pixels(rgba.len(), width, height)?;
        let source = ImageSource::from_bytes(rgba, (width, height)).map_err(|e| e.to_string())?;
        let input = self
            .engine
            .prepare_input(source)
            .map_err(|e| e.to_string())?;
        let words = self
            .engine
            .detect_words(&input)
            .map_err(|e| e.to_string())?;
        let lines = self.engine.find_text_lines(&input, &words);
        let texts = self
            .engine
            .recognize_text(&input, &lines)
            .map_err(|e| e.to_string())?;
        Ok(join_lines(
            texts.into_iter().flatten().map(|line| line.to_string()),
        ))
    }
}

/// Whether `len` bytes are exactly `width` × `height` RGBA pixels of a
/// picture that is not empty.
pub fn check_pixels(len: usize, width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err(format!("empty picture {width}×{height}"));
    }
    let wanted = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    match wanted {
        Some(wanted) if wanted == len => Ok(()),
        _ => Err(format!("{len} bytes are not {width}×{height} RGBA pixels")),
    }
}

/// The recognised `lines` as one text: each trimmed, blank ones left
/// out, one per line.
pub fn join_lines(lines: impl IntoIterator<Item = String>) -> String {
    lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A width or height from Java, which has no unsigned ints.
fn dimension(value: jint) -> Option<u32> {
    u32::try_from(value).ok()
}

/// `OcrNative.open(detection, recognition)`: a handle to a [Reader] with
/// the two models, or 0 when they could not be loaded. A panic never
/// crosses into Java.
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_OcrNative_open<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    detection: JByteArray<'local>,
    recognition: JByteArray<'local>,
) -> jlong {
    let reader = catch_unwind(AssertUnwindSafe(|| {
        let detection = env.convert_byte_array(&detection).ok()?;
        let recognition = env.convert_byte_array(&recognition).ok()?;
        Reader::new(detection, recognition).ok()
    }));
    match reader {
        Ok(Some(reader)) => Box::into_raw(Box::new(reader)) as jlong,
        _ => 0,
    }
}

/// `OcrNative.read(reader, rgba, width, height)`: the text in the
/// picture, empty when there is none, or null when it could not be read.
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_OcrNative_read<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    reader: jlong,
    rgba: JByteArray<'local>,
    width: jint,
    height: jint,
) -> jstring {
    if reader == 0 {
        return ptr::null_mut();
    }
    // SAFETY: a non-zero handle comes from `open` and stays valid until
    // `close`; the app uses it from one thread only.
    let reader = unsafe { &*(reader as *const Reader) };
    let text = catch_unwind(AssertUnwindSafe(|| {
        let pixels = env.convert_byte_array(&rgba).ok()?;
        reader
            .read(&pixels, dimension(width)?, dimension(height)?)
            .ok()
    }));
    match text {
        Ok(Some(text)) => env
            .new_string(text)
            .map(|text| text.into_raw())
            .unwrap_or(ptr::null_mut()),
        _ => ptr::null_mut(),
    }
}

/// `OcrNative.close(reader)`: frees the [Reader] and its models.
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_OcrNative_close<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    reader: jlong,
) {
    if reader == 0 {
        return;
    }
    // SAFETY: as in `read`; the app never uses the handle again.
    let _ = catch_unwind(|| drop(unsafe { Box::from_raw(reader as *mut Reader) }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The models exactly as the app bundles them.
    fn bundled(name: &str) -> Vec<u8> {
        std::fs::read(root().join("../app/src/main/assets/ocrs").join(name)).unwrap()
    }

    fn reader() -> Reader {
        Reader::new(
            bundled("text-detection.rten"),
            bundled("text-recognition.rten"),
        )
        .unwrap()
    }

    #[test]
    fn pixels_must_fill_the_picture_exactly() {
        assert_eq!(check_pixels(2 * 3 * 4, 2, 3), Ok(()));
        assert!(check_pixels(2 * 3 * 3, 2, 3).is_err());
        assert!(check_pixels(2 * 3 * 4 + 1, 2, 3).is_err());
        assert!(check_pixels(0, 0, 3).is_err());
        assert!(check_pixels(0, 3, 0).is_err());
        assert!(check_pixels(usize::MAX, u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn lines_are_trimmed_and_blank_ones_left_out() {
        let lines = ["  Milk ", "", "   ", "eggs\t"].map(String::from);
        assert_eq!(join_lines(lines), "Milk\neggs");
        assert_eq!(join_lines(Vec::<String>::new()), "");
    }

    #[test]
    fn negative_dimensions_are_refused() {
        assert_eq!(dimension(-1), None);
        assert_eq!(dimension(2048), Some(2048));
    }

    /// testdata/rust-book.jpg is copied unchanged from the examples of
    /// the ocrs 0.13.1 crate.
    #[test]
    fn the_bundled_models_read_a_photographed_page() {
        let photo = image::open(root().join("testdata/rust-book.jpg"))
            .unwrap()
            .into_rgba8();
        let (width, height) = photo.dimensions();
        let text = reader().read(photo.as_raw(), width, height).unwrap();
        assert!(text.contains("THE RUST\nPROGRAMMING\nLANGUAGE"), "{text}");
        assert!(text
            .lines()
            .all(|line| line == line.trim() && !line.is_empty()));
    }

    #[test]
    fn a_blank_picture_has_no_text() {
        let white = vec![255u8; 64 * 48 * 4];
        assert_eq!(reader().read(&white, 64, 48), Ok(String::new()));
    }

    #[test]
    fn pixels_that_do_not_fit_are_not_read() {
        assert!(reader().read(&[0; 10], 64, 48).is_err());
    }
}
