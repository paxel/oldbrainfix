//! Desktop notifications, as Catlog's desktop shows them: the system's
//! notification service through notify-rust outside tests, a recorder
//! inside. On Linux a notification offers its buttons (Done, the
//! snoozes, Open), shows again in place of the one with the same key,
//! and is closed once its Occurrence is Done. A Windows toast offers the
//! same buttons but is neither replaced nor closed: notify-rust shows
//! each one anew and closes none. On macOS a click on it is heard, but
//! it offers no buttons: notify-rust puts more than one under a menu
//! titled "Options" in English only.

use std::sync::{Arc, Mutex};

use crate::alerts::{Notice, Notifier};

/// The sound an escalated notification asks for (the freedesktop sound
/// naming spec's alarm).
pub const ALARM_SOUND: &str = "alarm-clock-elapsed";

/// The platform notification service; what is tapped on a notification
/// goes to `taps`.
pub struct DesktopNotifier {
    taps: std::sync::mpsc::Sender<crate::alerts::Event>,
    /// The id the notification service gave each key's notification,
    /// and whether a thread still waits for its buttons. One shown again
    /// in place keeps its id, and its waiter hears the new one's buttons.
    #[cfg(all(unix, not(target_os = "macos")))]
    ids: std::collections::HashMap<String, (u32, Arc<std::sync::atomic::AtomicBool>)>,
}

impl DesktopNotifier {
    pub fn new(taps: std::sync::mpsc::Sender<crate::alerts::Event>) -> DesktopNotifier {
        DesktopNotifier {
            taps,
            #[cfg(all(unix, not(target_os = "macos")))]
            ids: std::collections::HashMap::new(),
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl Notifier for DesktopNotifier {
    fn show(&mut self, notice: &Notice) {
        use notify_rust::{Hint, Notification, Urgency};
        let mut n = Notification::new();
        n.summary(&notice.title)
            .body(&notice.body)
            .appname(oldbrainfix_core::APP_NAME)
            .icon(crate::ICON_NAME)
            .hint(Hint::DesktopEntry(oldbrainfix_core::APP_ID.into()));
        for (id, label) in &notice.actions {
            n.action(id, label);
        }
        if notice.urgent {
            n.urgency(Urgency::Critical).sound_name(ALARM_SOUND);
        }
        if notice.silent {
            n.hint(Hint::SuppressSound(true));
        }
        let held = self.ids.get(&notice.key).cloned();
        if let Some((id, _)) = &held {
            n.id(*id);
        }
        match n.show() {
            Ok(handle) => {
                use std::sync::atomic::{AtomicBool, Ordering};
                let id = handle.id();
                if let Some((held_id, waiting)) = &held
                    && *held_id == id
                    && waiting.load(Ordering::SeqCst)
                {
                    return;
                }
                let waiting = Arc::new(AtomicBool::new(true));
                self.ids.insert(notice.key.clone(), (id, waiting.clone()));
                let taps = self.taps.clone();
                let key = notice.key.clone();
                std::thread::spawn(move || {
                    let waiter = waiting.clone();
                    handle.wait_for_action(|action| {
                        if action != "__closed" {
                            let tap = crate::alerts::Tap {
                                key,
                                action: action.to_string(),
                                at: chrono::Utc::now(),
                            };
                            drop(taps.send(crate::alerts::Event::Tap(tap)));
                        }
                    });
                    waiter.store(false, Ordering::SeqCst);
                });
            }
            Err(e) => eprintln!("oldbrainfix: notification: {e}"),
        }
    }

    fn close(&mut self, key: &str) {
        let Some((id, _)) = self.ids.remove(key) else {
            return;
        };
        let closed = zbus::blocking::Connection::session().and_then(|bus| {
            bus.call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "CloseNotification",
                &(id,),
            )
        });
        if let Err(e) = closed {
            eprintln!("oldbrainfix: closing a notification: {e}");
        }
    }
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
impl Notifier for DesktopNotifier {
    fn show(&mut self, notice: &Notice) {
        use notify_rust::NotificationResponse;
        let mut n = notify_rust::Notification::new();
        n.summary(&notice.title)
            .body(&notice.body)
            .appname(oldbrainfix_core::APP_NAME);
        if cfg!(target_os = "windows") {
            for (id, label) in &notice.actions {
                n.action(id, label);
            }
        }
        // Without a sound name a toast and a macOS notification stay
        // silent, as a `silent` notice is to.
        if notice.urgent {
            n.sound_name("Default");
        }
        // A toast is shown here; a macOS notification only once its
        // handle waits, on this thread, for a click or a button.
        let handle = match n.show() {
            Ok(handle) => handle,
            Err(e) => {
                eprintln!("oldbrainfix: notification: {e}");
                return;
            }
        };
        let taps = self.taps.clone();
        let key = notice.key.clone();
        std::thread::spawn(move || {
            let waited = handle.wait_for_response(|response: &NotificationResponse| {
                // A click on the notification itself is Open.
                let action = match response {
                    NotificationResponse::Default => crate::alerts::OPEN.to_string(),
                    NotificationResponse::Action(id) => id.clone(),
                    _ => return,
                };
                let tap = crate::alerts::Tap {
                    key,
                    action,
                    at: chrono::Utc::now(),
                };
                drop(taps.send(crate::alerts::Event::Tap(tap)));
            });
            if let Err(e) = waited {
                eprintln!("oldbrainfix: notification: {e}");
            }
        });
    }

    fn close(&mut self, _key: &str) {}
}

/// What a [`RecordingNotifier`] was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorded {
    Shown(Notice),
    Closed(String),
}

/// Keeps what would have been shown and closed, readable from outside.
#[derive(Debug, Default, Clone)]
pub struct RecordingNotifier {
    pub record: Arc<Mutex<Vec<Recorded>>>,
}

impl RecordingNotifier {
    /// Everything recorded so far, taken out.
    pub fn take(&self) -> Vec<Recorded> {
        self.record
            .lock()
            .map(|mut r| std::mem::take(&mut *r))
            .unwrap_or_default()
    }
}

impl Notifier for RecordingNotifier {
    fn show(&mut self, notice: &Notice) {
        if let Ok(mut record) = self.record.lock() {
            record.push(Recorded::Shown(notice.clone()));
        }
    }

    fn close(&mut self, key: &str) {
        if let Ok(mut record) = self.record.lock() {
            record.push(Recorded::Closed(key.to_string()));
        }
    }
}
