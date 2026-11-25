use super::*;

fn ended(said: &str) -> Ended {
    Ended {
        id: JobId::Import,
        what: "job".into(),
        said: said.into(),
        goes_on: false,
    }
}

#[test]
fn the_stopped_jobs_are_listed_newest_first_and_at_most_ten() {
    let mut stopped = Stopped::default();
    for n in 0..12 {
        stopped.push(ended(&n.to_string()));
    }
    let said: Vec<&str> = stopped.list().iter().map(|e| e.said.as_str()).collect();
    assert_eq!(said.len(), MAX_ENDED);
    assert_eq!(said.first(), Some(&"11"));
    assert_eq!(said.last(), Some(&"2"));
}
