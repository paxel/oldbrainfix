use super::*;

#[test]
fn ascii85_reads_as_the_spec_writes_it() {
    assert_eq!(ascii85(b"<~87cURD]i,\"Ebo80~>").unwrap(), b"Hello World!");
    assert_eq!(ascii85(b"z~>").unwrap(), vec![0; 4]);
    assert_eq!(ascii_hex(b"48 65 6c6C6f7>"), b"Hellop");
}

#[test]
fn png_predictors_are_undone() {
    // Two rows of three bytes: Sub, then Up.
    let data = vec![1, 10, 5, 5, 2, 1, 1, 1];
    let parms = Dict(vec![
        (b"Predictor".to_vec(), Obj::Int(12)),
        (b"Columns".to_vec(), Obj::Int(3)),
    ]);
    assert_eq!(
        predicted(data, Some(&parms)).unwrap(),
        vec![10, 15, 20, 11, 16, 21]
    );
}
