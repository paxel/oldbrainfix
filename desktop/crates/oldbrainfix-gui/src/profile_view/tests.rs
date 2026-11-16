use super::*;

#[test]
fn a_copied_id_and_version_with_another_picture_is_another_texture() {
    let profile = |picture: &[u8]| Profile {
        id: "ann".into(),
        name: "Ann".into(),
        picture: Some(picture.to_vec()),
        version: 1,
    };
    assert_eq!(picture_key(&profile(b"one")), picture_key(&profile(b"one")));
    assert_ne!(picture_key(&profile(b"one")), picture_key(&profile(b"two")));
}
