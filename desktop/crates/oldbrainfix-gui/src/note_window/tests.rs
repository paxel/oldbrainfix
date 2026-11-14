use super::*;

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        vars.iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    }
}

#[test]
fn on_wayland_no_window_is_kept_on_top() {
    let wayland = pin_works(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]));
    let x11 = pin_works(env(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", ":0")]));
    if cfg!(target_os = "linux") {
        assert!(!wayland);
    } else {
        assert!(wayland);
    }
    assert!(x11);
}

#[test]
fn the_pin_puts_the_window_over_the_others() {
    let size = Vec2::new(500.0, 600.0);
    let normal = viewport("Recipe", size, false);
    assert_eq!(normal.window_level, Some(WindowLevel::Normal));
    assert_eq!(normal.inner_size, Some(size));
    assert_eq!(normal.title.as_deref(), Some("Recipe"));
    let pinned = viewport("Recipe", size, true);
    assert_eq!(pinned.window_level, Some(WindowLevel::AlwaysOnTop));
}

#[test]
fn a_pin_where_it_cannot_work_keeps_nothing_on_top() {
    let mut alone = Alone::of("n1", false);
    alone.on_top = true;
    assert!(!alone.kept_on_top());
    let mut alone = Alone::of("n1", true);
    assert!(!alone.kept_on_top());
    alone.on_top = true;
    assert!(alone.kept_on_top());
}

#[test]
fn the_title_is_the_first_line() {
    assert_eq!(title("\n  Pancakes \n200 g flour", "App"), "Pancakes");
    assert_eq!(title("  ", "App"), "App");
    let long = "a".repeat(80);
    let shown = title(&long, "App");
    assert_eq!(shown.chars().count(), TITLE_CHARS);
    assert!(shown.ends_with('…'));
}

#[test]
fn each_window_has_a_viewport_of_its_own() {
    assert_ne!(viewport_id(1), viewport_id(2));
    assert_ne!(viewport_id(1), ViewportId::ROOT);
}
