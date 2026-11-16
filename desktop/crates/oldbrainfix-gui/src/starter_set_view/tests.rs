use super::*;

#[test]
fn an_area_unticked_takes_its_topics_and_a_topic_ticked_brings_its_area() {
    let mut picked = all();
    assert!(picked.contains("household") && picked.contains("kitchen"));
    tick_area(&mut picked, "household", false);
    for slug in ["household", "kitchen", "garden", "cleaning"] {
        assert!(!picked.contains(slug), "{slug}");
    }
    assert!(picked.contains("work"));
    tick_topic(&mut picked, "household", "garden", true);
    assert!(picked.contains("household") && picked.contains("garden"));
    assert!(!picked.contains("kitchen"));
    tick_topic(&mut picked, "household", "garden", false);
    assert!(picked.contains("household") && !picked.contains("garden"));
}
