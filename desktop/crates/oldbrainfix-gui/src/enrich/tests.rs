use super::*;

#[test]
fn lines_are_trimmed_and_blank_ones_left_out() {
    let lines = ["  Milk ", "", "   ", "eggs\t"].map(String::from);
    assert_eq!(join_lines(lines), "Milk\neggs");
    assert_eq!(join_lines(Vec::<String>::new()), "");
}

/// testdata/rust-book.jpg of android/ocr, copied there unchanged
/// from the examples of the ocrs 0.13.1 crate.
#[test]
fn the_phones_models_read_a_photographed_page() {
    let photo = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../android/ocr/testdata/rust-book.jpg"
    ))
    .unwrap_or_default();
    let text = OcrEngine::default().read(photo).unwrap_or_default();
    assert!(text.contains("THE RUST\nPROGRAMMING\nLANGUAGE"), "{text}");
}

/// A photo of android/ocr's testdata, read as the desktop reads it.
fn read_testdata(name: &str) -> String {
    let path = format!(
        "{}/../../../android/ocr/testdata/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let photo = std::fs::read(path).unwrap_or_default();
    OcrEngine::default().read(photo).unwrap_or_default()
}

#[test]
fn german_letters_and_the_euro_sign_are_read() {
    let text = read_testdata("german-receipt.jpg");
    assert!(text.contains("Bäckerei Müller GmbH"), "{text}");
    assert!(text.contains("Hauptstraße 7, 80331 München"), "{text}");
    assert!(
        text.lines()
            .any(|line| line.starts_with("Käsekuchen") && line.ends_with("3,90 €")),
        "{text}"
    );
}

/// With RustO's own box threshold of 0.6 seven of the page's twelve
/// lines are gone.
#[test]
fn every_line_of_a_curved_page_is_read() {
    let text = read_testdata("curved-page.jpg");
    assert_eq!(text.lines().count(), 12, "{text}");
    assert!(text.contains("once or twice"), "{text}");
    assert!(text.contains("queue full"), "{text}");
}

/// The box threshold must sit at the top level: RustO 0.3.2 ignores
/// `detection.postprocess`.
#[test]
fn spatial_text_with_the_lower_box_threshold() {
    let options = ocr_options();
    assert_eq!(options.output, rusto::OutputGranularity::Spatial);
    assert!(options.detection.is_none());
    let box_threshold = options.postprocess.and_then(|p| p.box_threshold);
    assert_eq!(box_threshold, Some(0.3));
}

#[test]
fn what_is_no_picture_is_not_read() {
    assert_eq!(OcrEngine::default().read(b"no picture".to_vec()), None);
}

/// The stores of the prompt fixture the Dart core wrote, seen with
/// the app's own strings, give the prompts it wrote: the Area and
/// Topic names as the user reads them, translated Starter Set places
/// in German among them.
#[test]
fn the_prompts_name_the_places_as_the_app_shows_them() -> oldbrainfix_core::Result<()> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/transcript_prompts.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let fixture: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let views = fixture["views"].as_array().cloned().unwrap_or_default();
    assert_eq!(views.len(), 6);
    for view in views {
        let name = view["store"].as_str().unwrap_or_default();
        let t = L10n::new(view["locale"].as_str().unwrap_or_default());
        let store = Store::in_memory(Box::new(chrono::Utc::now), || {
            Ok(oldbrainfix_core::SigningKey::from_seed([1; 32]))
        })?;
        for entry in fixture["stores"][name].as_array().into_iter().flatten() {
            if let Some(entry) = oldbrainfix_core::WireEntry::from_value(entry) {
                store.insert(&entry)?;
            }
        }
        let prompts = transcript_prompts(&store, &t);
        assert_eq!(prompts.english, view["english"], "{name}");
        assert_eq!(prompts.german, view["german"], "{name}");
    }
    let de = L10n::new("de");
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(oldbrainfix_core::SigningKey::from_seed([1; 32]))
    })?;
    let places = Places::new(&store);
    places.plant_starter_set(&["household", "kitchen"])?;
    places.add_area("Fahrrad")?;
    let prompts = transcript_prompts(&store, &de);
    assert_eq!(prompts.english, "Fahrrad, Haushalt, Küche.");
    // What the Transcript is guarded against.
    assert_eq!(prompts.names, ["Fahrrad", "Haushalt", "Küche"]);
    assert_eq!(prompts.terms, ["Unsortiert", "Routine", "Agenda"]);
    assert_eq!(prompts.guard("Fahrrad, Haushalt, Küche."), "");
    assert!(
        prompts
            .german
            .ends_with("Unsortiert, Routine, Agenda. Fahrrad, Haushalt, Küche.")
    );
    Ok(())
}

#[test]
fn without_its_model_whisper_reads_nothing() {
    let mut engine = WhisperEngine::new(
        PathBuf::from("/nonexistent/ggml-base.bin"),
        Box::new(TranscriptPrompts::default),
    );
    assert_eq!(engine.read(crate::media::wav(&[0.0; 1600])), None);
    assert_eq!(engine.read(b"no recording".to_vec()), None);
}
