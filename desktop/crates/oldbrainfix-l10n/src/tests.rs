use super::*;

const EN: &str = r#"{
  "@@locale": "en",
  "appTitle": "cat(a)log",
  "csvSavedTo": "CSV saved to {path}",
  "@csvSavedTo": {"placeholders": {"path": {"type": "String"}}},
  "changesCount": "{n, plural, one{1 change} other{{n} changes}}",
  "@changesCount": {"placeholders": {"n": {"type": "int"}}},
  "greet": "Hello {name}, you have {count}",
  "type": "Kind",
  "gender": "{g, select, female{she} male{he} other{they}}"
}"#;
const DE: &str = r#"{
  "@@locale": "de",
  "appTitle": "cat(a)log",
  "csvSavedTo": "CSV gespeichert unter {path}",
  "changesCount": "{n, plural, one{1 Änderung} other{{n} Änderungen}}"
}"#;

#[test]
fn arb_files_parse_with_their_placeholders() {
    let en = parse_arb(EN).unwrap();
    assert_eq!(en.locale, "en");
    assert_eq!(en.messages.len(), 6);
    assert_eq!(
        en.placeholders["csvSavedTo"],
        vec![("path".to_string(), "String".to_string())]
    );
    assert!(parse_arb("[]").is_err());
    assert!(parse_arb(r#"{"a":"b"}"#).is_err());
    assert!(parse_arb("not json").is_err());
}

#[test]
fn keys_become_snake_case_and_keywords_get_an_underscore() {
    assert_eq!(snake("csvSavedTo"), "csv_saved_to");
    assert_eq!(snake("type"), "type_");
    assert_eq!(snake("appTitle"), "app_title");
}

#[test]
fn the_generated_source_has_a_table_and_typed_accessors() {
    let arbs = vec![parse_arb(DE).unwrap(), parse_arb(EN).unwrap()];
    let src = generate(&arbs, "en").unwrap();
    assert!(src.contains(r#"pub const LOCALES: [&str; 2] = ["en", "de", ];"#));
    assert!(src.contains("pub fn app_title(&self) -> &'static str { self.messages()[0] }"));
    assert!(src.contains("pub fn csv_saved_to(&self, path: &str) -> String"));
    assert!(src.contains("pub fn changes_count(&self, n: i64) -> String"));
    assert!(src.contains("pub fn greet(&self, name: &str, count: &str) -> String"));
    assert!(src.contains("pub fn gender(&self, g: &str) -> String"));
    assert!(src.contains("pub fn type_(&self) -> &'static str"));
    // German falls back to English where it has no text of its own.
    assert!(src.contains(r#""Hello {name}, you have {count}", "#));
    assert!(src.matches("Hello {name}").count() == 2);
    assert!(generate(&arbs, "fr").is_err());
}

#[test]
fn a_directory_of_arb_files_is_read_in_order() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app_en.arb"), EN).unwrap();
    std::fs::write(dir.path().join("app_de.arb"), DE).unwrap();
    std::fs::write(dir.path().join("other.arb"), "junk").unwrap();
    std::fs::write(dir.path().join("app_en.dart"), "junk").unwrap();
    let arbs = read_dir(dir.path()).unwrap();
    assert_eq!(
        arbs.iter().map(|a| a.locale.as_str()).collect::<Vec<_>>(),
        vec!["de", "en"]
    );
    assert!(generate_dir(dir.path(), "en").is_ok());
    // An overlay directory adds and overrides keys per locale.
    let overlay = tempfile::tempdir().unwrap();
    std::fs::write(
        overlay.path().join("desktop_en.arb"),
        r#"{"@@locale": "en", "menuFile": "File", "appTitle": "cat(a)log desktop"}"#,
    )
    .unwrap();
    std::fs::write(
        overlay.path().join("desktop_fr.arb"),
        r#"{"@@locale": "fr", "menuFile": "Fichier"}"#,
    )
    .unwrap();
    let merged = read_dirs(&[dir.path(), overlay.path()]).unwrap();
    assert_eq!(merged.len(), 3);
    let en = merged.iter().find(|a| a.locale == "en").unwrap();
    assert_eq!(en.messages["menuFile"], "File");
    assert_eq!(en.messages["appTitle"], "cat(a)log desktop");
    let src = generate_dirs(&[dir.path(), overlay.path()], "en").unwrap();
    assert!(src.contains("pub fn menu_file(&self)"));
    std::fs::write(dir.path().join("app_xx.arb"), "junk").unwrap();
    assert!(read_dir(dir.path()).is_err());
    assert!(read_dir(&dir.path().join("missing")).is_err());
}
