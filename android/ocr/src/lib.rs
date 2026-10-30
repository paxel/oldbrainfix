//! Text in photos for Old Brain Fix on Android (ADR 0007): RustO with
//! the PP-OCRv6 Tiny detection and recognition models and their
//! character list bundled in the app, behind three JNI functions for
//! `OcrNative` in the app's Kotlin. Fully offline, no Google library.
//!
//! The app decodes the photo itself (an upright JPEG of at most 2048
//! px) and hands in its RGBA pixels and the bytes of the model files, so
//! neither an image decoder nor a file path is needed.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use jni::objects::{JByteArray, JObject};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use ndarray::ArrayD;
use rten::{Model, Value};
use rten_tensor::prelude::*;
use rten_tensor::Tensor;
use rusto::engine::{EngineError, InferenceSession};
use rusto::{
    DetectTextResult, InitializeConfig, OcrRunOptions, OutputGranularity, PostprocessRunOptions,
    RustO,
};

/// RustO with both models loaded, ready to read photos.
pub struct Reader {
    engine: RustO,
}

impl Reader {
    /// Loads the text-detection and text-recognition models from the
    /// bytes of their `.rten` files, and the recognition model's
    /// characters from its `dict.txt`, one per line.
    pub fn new(
        detection: Vec<u8>,
        recognition: Vec<u8>,
        characters: &str,
    ) -> Result<Reader, String> {
        let detection = Session::new(detection, None)?;
        let recognition = Session::new(recognition, Some(character_list(characters)))?;
        // The PP-OCRv6 settings; the paths are not read, the sessions
        // hold the models.
        let config = InitializeConfig::ppv6("det.rten", "rec.rten", "dict.txt");
        let engine = RustO::with_custom_engines(config, detection, recognition)
            .map_err(|e| e.to_string())?;
        Ok(Reader { engine })
    }

    /// The text in a picture of `width` × `height` RGBA pixels, row by
    /// row: one line per line of text, top to bottom, words on one line
    /// spaced as on the picture; empty when there is none.
    pub fn read(&mut self, rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
        check_pixels(rgba.len(), width, height)?;
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect();
        let stride = width as usize * 3;
        let result = self
            .engine
            .detect_text_rgb(&rgb, width, height, stride, &options())
            .map_err(|e| e.to_string())?;
        Ok(match result {
            DetectTextResult::Spatial(text) => join_lines(text.lines().map(String::from)),
            DetectTextResult::Structured(lines) => {
                join_lines(lines.into_iter().map(|line| line.text))
            }
        })
    }
}

/// How RustO reads (ADR 0007): the text laid out as on the picture, and
/// a box kept from a probability of 0.3 instead of 0.6, so the lines of
/// a curved page are not dropped. The threshold is the top-level
/// `postprocess`; RustO 0.3.2 ignores `detection.postprocess`
/// (https://github.com/byrizki/rusto-rs/issues/21).
pub fn options() -> OcrRunOptions {
    OcrRunOptions {
        output: OutputGranularity::Spatial,
        postprocess: Some(PostprocessRunOptions {
            box_threshold: Some(BOX_THRESHOLD),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// The detection score from which a box of text is read.
pub const BOX_THRESHOLD: f32 = 0.3;

/// The characters of `dict.txt`, one per line, as RustO reads the file.
pub fn character_list(dict: &str) -> Vec<String> {
    dict.lines().map(String::from).collect()
}

/// One model run as RustO runs its own models from a file, but loaded
/// from bytes: the first input in, the first output out. The recognition
/// model also holds its characters.
struct Session {
    model: Model,
    characters: Option<Vec<String>>,
}

impl Session {
    fn new(bytes: Vec<u8>, characters: Option<Vec<String>>) -> Result<Session, String> {
        let model = Model::load(bytes).map_err(|e| e.to_string())?;
        Ok(Session { model, characters })
    }
}

impl InferenceSession for Session {
    fn run(&mut self, input: ArrayD<f32>) -> Result<ArrayD<f32>, EngineError> {
        let shape = input.shape().to_vec();
        let tensor = Tensor::<f32>::from_data(&shape, input.iter().copied().collect::<Vec<_>>());
        let input_id = *self
            .model
            .input_ids()
            .first()
            .ok_or_else(|| EngineError::OutputError("Model has no inputs".into()))?;
        let outputs = self.model.run(
            vec![(input_id, tensor.view().into())],
            self.model.output_ids(),
            None,
        )?;
        match outputs.first() {
            Some(Value::FloatTensor(output)) => Ok(ArrayD::from_shape_vec(
                output.shape().to_vec(),
                output.iter().copied().collect(),
            )?),
            Some(_) => Err(EngineError::OutputError(
                "Unsupported output tensor type".into(),
            )),
            None => Err(EngineError::OutputError("Model produced no outputs".into())),
        }
    }

    fn run_all(
        &mut self,
        _input: ArrayD<f32>,
    ) -> Result<HashMap<String, ArrayD<f32>>, EngineError> {
        Err(EngineError::Engine("not used for text".into()))
    }

    fn run_with_inputs(
        &mut self,
        _inputs: HashMap<String, ArrayD<f32>>,
    ) -> Result<HashMap<String, ArrayD<f32>>, EngineError> {
        Err(EngineError::Engine("not used for text".into()))
    }

    fn get_character_list(&self, key: &str) -> Option<Vec<String>> {
        (key == "character")
            .then(|| self.characters.clone())
            .flatten()
    }

    fn have_key(&self, key: &str) -> bool {
        key == "character" && self.characters.is_some()
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

/// `OcrNative.open(detection, recognition, characters)`: a handle to a
/// [Reader] with the two models and the characters of `dict.txt`, or 0
/// when they could not be loaded. A panic never crosses into Java.
#[no_mangle]
pub extern "system" fn Java_io_github_paxel_oldbrainfix_OcrNative_open<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    detection: JByteArray<'local>,
    recognition: JByteArray<'local>,
    characters: JByteArray<'local>,
) -> jlong {
    let reader = catch_unwind(AssertUnwindSafe(|| {
        let detection = env.convert_byte_array(&detection).ok()?;
        let recognition = env.convert_byte_array(&recognition).ok()?;
        let characters = String::from_utf8(env.convert_byte_array(&characters).ok()?).ok()?;
        Reader::new(detection, recognition, &characters).ok()
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
    // `close`; the app uses it from one thread only, so this is the only
    // reference to the Reader while it reads.
    let reader = unsafe { &mut *(reader as *mut Reader) };
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

    /// The model files exactly as the app bundles them.
    fn bundled(name: &str) -> Vec<u8> {
        std::fs::read(root().join("../app/src/main/assets/rusto").join(name)).unwrap()
    }

    fn reader() -> Reader {
        let characters = String::from_utf8(bundled("dict.txt")).unwrap();
        Reader::new(bundled("det.rten"), bundled("rec.rten"), &characters).unwrap()
    }

    fn read(photo: &str) -> String {
        let photo = image::open(root().join("testdata").join(photo))
            .unwrap()
            .into_rgba8();
        let (width, height) = photo.dimensions();
        reader().read(photo.as_raw(), width, height).unwrap()
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

    #[test]
    fn the_characters_are_the_lines_of_the_dictionary() {
        assert_eq!(character_list("a\nß\r\n€\n"), ["a", "ß", "€"]);
    }

    /// The box threshold must sit at the top level: RustO 0.3.2 ignores
    /// `detection.postprocess`.
    #[test]
    fn spatial_text_with_the_lower_box_threshold() {
        let options = options();
        assert_eq!(options.output, OutputGranularity::Spatial);
        assert!(options.detection.is_none());
        let postprocess = options.postprocess.unwrap();
        assert_eq!(postprocess.box_threshold, Some(0.3));
        assert_eq!(postprocess.threshold, None);
    }

    /// testdata/rust-book.jpg is copied unchanged from the examples of
    /// the ocrs 0.13.1 crate.
    #[test]
    fn the_bundled_models_read_a_photographed_page() {
        let text = read("rust-book.jpg");
        assert!(text.contains("THE RUST\nPROGRAMMING\nLANGUAGE"), "{text}");
        assert!(text
            .lines()
            .all(|line| line == line.trim() && !line.is_empty()));
    }

    /// testdata/german-receipt.jpg is rendered for #118 in Courier
    /// Prime; ocrs wrote "?" and "E" for its letters.
    #[test]
    fn german_letters_and_the_euro_sign_are_read() {
        let text = read("german-receipt.jpg");
        assert!(text.contains("Bäckerei Müller GmbH"), "{text}");
        assert!(text.contains("Hauptstraße 7, 80331 München"), "{text}");
        assert!(text.contains("für"), "{text}");
        // A price stays on the line of its item.
        assert!(
            text.lines()
                .any(|line| line.starts_with("Käsekuchen") && line.ends_with("3,90 €")),
            "{text}"
        );
    }

    /// testdata/curved-page.jpg is the page of the RustO issue on lines
    /// lost on curved pages (#118): with RustO's own box threshold of 0.6
    /// seven of its twelve lines are gone.
    #[test]
    fn every_line_of_a_curved_page_is_read() {
        let text = read("curved-page.jpg");
        assert_eq!(text.lines().count(), 12, "{text}");
        for (line, part) in text.lines().zip([
            "Alice was",
            "RUN_EXAMPLE 3",
            "once or twice",
            "book her",
            "thought",
            "considering in her own mind",
            "queue full",
            "BoundedBuffer",
            "previously reported",
            "rabbit/Watch",
            "daisy-chain",
            "picking the daisies",
        ]) {
            assert!(line.contains(part), "{part}: {text}");
        }
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
