use super::*;

#[test]
fn every_form_reads_back_from_the_link_it_writes() {
    for link in [
        AppLink::Note("note:3f2a".into()),
        AppLink::Place("topic:x y/ü".into()),
        AppLink::Agenda,
        AppLink::Unsorted,
        AppLink::Settings(None),
        AppLink::Settings(Some("alarms".into())),
        AppLink::Routines,
        AppLink::Backups,
        AppLink::Sync,
        AppLink::Profile,
        AppLink::Frogs,
        AppLink::NewSession,
        AppLink::Guide(None, None),
        AppLink::Guide(Some("getting-started".into()), Some("set-up".into())),
        AppLink::File("Scans/Tax 2026 (a).pdf".into()),
    ] {
        assert_eq!(parse(&link.url()), Some(link.clone()), "{}", link.url());
    }
    assert_eq!(
        AppLink::Place("topic:x y/ü".into()).url(),
        "oldbrainfix://area/topic:x%20y%2F%C3%BC"
    );
    assert_eq!(parse("OldBrainFix://agenda"), Some(AppLink::Agenda));
}

#[test]
fn a_file_link_never_leads_out_of_the_shared_folder() {
    for url in [
        "oldbrainfix://file/../etc/passwd",
        "oldbrainfix://file/a/%2E%2E/b",
        "oldbrainfix://file//etc/passwd",
        "oldbrainfix://file/a%2Fb",
        "oldbrainfix://file/a%5Cb",
        "oldbrainfix://file/C%3A",
        "oldbrainfix://file/a%00",
        "oldbrainfix://file",
    ] {
        assert_eq!(parse(url), None, "{url}");
    }
}

#[test]
fn anything_else_with_the_scheme_is_no_app_link() {
    for url in [
        "oldbrainfix://x",
        "oldbrainfix://capture/voice",
        "oldbrainfix://pick?session=s",
        "oldbrainfix://agenda/",
        "oldbrainfix://agenda#x",
        "oldbrainfix://note/%C3",
        "oldbrainfix://settings/nothing",
        "oldbrainfix://guide/a#B",
        "oldbrainfix:ü",
    ] {
        assert_eq!(parse(url), None, "{url}");
    }
    assert_eq!(
        AppLink::Guide(Some("a".into()), None).guide_address("de"),
        Some("https://paxel.github.io/oldbrainfix/de/guide/a.html".into())
    );
}
