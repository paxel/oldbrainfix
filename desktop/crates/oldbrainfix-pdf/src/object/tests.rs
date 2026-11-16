use super::*;

fn all(text: &[u8]) -> Vec<Obj> {
    let mut lexer = Lexer::new(text, 0);
    std::iter::from_fn(|| lexer.object()).collect()
}

#[test]
fn objects_read_as_written() {
    let read = all(b"<< /Type /Page /Kids [1 0 R 2 0 R] /N -3 /R .5 /S (a\\(b\\)\\101\\\nc) /H <4142 3> /E#20x true >> null BT");
    let Obj::Dict(dict) = &read[0] else { panic!() };
    assert_eq!(dict.get(b"Type"), Some(&Obj::Name(b"Page".to_vec())));
    assert_eq!(
        dict.get(b"Kids"),
        Some(&Obj::Array(vec![Obj::Ref(1, 0), Obj::Ref(2, 0)]))
    );
    assert_eq!(dict.get(b"N"), Some(&Obj::Int(-3)));
    assert_eq!(dict.get(b"R"), Some(&Obj::Real(0.5)));
    assert_eq!(dict.get(b"S"), Some(&Obj::Str(b"a(b)Ac".to_vec())));
    assert_eq!(dict.get(b"H"), Some(&Obj::Str(b"AB0".to_vec())));
    assert_eq!(dict.get(b"E x"), Some(&Obj::Bool(true)));
    assert_eq!(read[1], Obj::Null);
    assert_eq!(read[2], Obj::Keyword(b"BT".to_vec()));
}

#[test]
fn what_nests_too_deep_does_not_read() {
    let deep = "[".repeat(1000);
    assert_eq!(all(deep.as_bytes()), Vec::new());
}
