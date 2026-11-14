use super::*;

#[test]
fn place_writes_check_what_they_write_to() -> Result<()> {
    let store = crate::store::tests::store();
    let places = Places::new(&store);
    assert!(places.add_area("  ").is_err());
    let area = places.add_area(" Home ")?;
    assert_eq!(area.name, "Home");
    assert!(places.add_topic("area:none", "x").is_err());
    let topic = places.add_topic(&area.id, "Post")?;
    assert!(places.add_topic(&topic.id, "Third level").is_err());
    assert!(places.rename("area:none", "x").is_err());
    assert!(places.set_icon(&area.id, Some("rocket")).is_err());
    assert!(places.set_colour(&topic.id, Some("C62828")).is_err());
    assert!(places.delete_area(&area.id, NotesGo::Area).is_err());
    assert!(places.delete_topic(&area.id, NotesGo::Unsorted).is_err());
    places.set_icon(&area.id, Some("home"))?;
    assert_eq!(
        places.area_by_id(&area.id)?.and_then(|a| a.icon),
        Some("home".into())
    );
    places.plant_starter_set(&["work", "meetings", "doctor"])?;
    let names: Vec<_> = places.areas()?.into_iter().map(|a| a.name).collect();
    assert_eq!(names, ["Home", "Work"]);
    places.delete_area(&area.id, NotesGo::Unsorted)?;
    assert!(places.topic_by_id(&topic.id)?.is_none());
    Ok(())
}

#[test]
fn starter_names_are_found_by_slug() {
    assert_eq!(starter_name("to-sort-out"), Some("To sort out"));
    assert_eq!(starter_name("health"), Some("Health"));
    assert_eq!(starter_name("nope"), None);
}
