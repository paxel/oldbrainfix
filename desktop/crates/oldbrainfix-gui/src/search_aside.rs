//! A search run on a thread of its own once typing stops (obf-1068, C11):
//! every letter shows in its field at once, the search follows about
//! [`PAUSE`] after the last one, and one search runs at a time. What a
//! search brings for a text no longer typed is dropped, the text typed
//! now searched after it.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

/// How long typing rests before its text is searched.
pub const PAUSE: f64 = 0.3;

/// How often the window looks whether the search running is done.
const POLL: Duration = Duration::from_millis(30);

/// What a search does on its thread.
pub type Work<T> = Box<dyn FnOnce() -> T + Send>;

/// The search of one field: what was searched last, the text seen last
/// and since when, and the search running, with the text it searches.
pub struct Searching<T> {
    searched: String,
    seen: String,
    since: f64,
    running: Option<(String, Receiver<T>)>,
}

impl<T> Default for Searching<T> {
    fn default() -> Self {
        Searching {
            searched: String::new(),
            seen: String::new(),
            since: 0.0,
            running: None,
        }
    }
}

impl<T: Send + 'static> Searching<T> {
    /// The text the results shown were found for.
    pub fn searched(&self) -> &str {
        &self.searched
    }

    /// Takes `query` as searched: its results show already.
    pub fn set_searched(&mut self, query: &str) {
        self.searched = query.to_string();
    }

    /// Whether a search runs.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Looks once per frame at the field holding `query`: what a search
    /// brought for it, with the text it searched, once it is in; a search
    /// of `query` started with `start` where it differs from what was
    /// searched, the typing rested [`PAUSE`] and none runs. `start` gives
    /// the work for the thread, none where it cannot run. The window
    /// looks again when either is due.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        query: &str,
        start: impl FnOnce(&str) -> Option<Work<T>>,
    ) -> Option<(String, T)> {
        let now = ctx.input(|i| i.time);
        if self.seen != query {
            self.seen = query.to_string();
            self.since = now;
        }
        if let Some((asked, arrives)) = &self.running {
            match arrives.try_recv() {
                Ok(found) => {
                    let asked = asked.clone();
                    self.running = None;
                    ctx.request_repaint();
                    if asked == query {
                        return Some((asked, found));
                    }
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(POLL);
                    return None;
                }
                Err(TryRecvError::Disconnected) => self.running = None,
            }
        }
        if query == self.searched {
            return None;
        }
        let left = self.since + PAUSE - now;
        if left > 0.0 {
            ctx.request_repaint_after(Duration::from_secs_f64(left));
            return None;
        }
        let work = start(query)?;
        let (sent, arrives) = std::sync::mpsc::channel();
        std::thread::spawn(move || drop(sent.send(work())));
        self.running = Some((query.to_string(), arrives));
        ctx.request_repaint_after(POLL);
        None
    }
}
