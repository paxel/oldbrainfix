//! Markdown files opened with the app from the Finder on macOS (Q12):
//! "Open with" and a double click on a file whose program it is hand the
//! files over as the open documents Apple Event (`kAEOpenDocuments`), at
//! the launch it causes and while the app runs. The handler goes in as
//! the app is about to finish launching, where AppKit's own ones are in
//! place and the event of the launch has not come yet; each file goes on
//! as one a later start hands over ([`crate::instance::file_handed`]).
//! The bundle offers the app for `.md` and `.markdown` files, as an
//! Alternate handler, never their default (`packaging/macos/pack.sh`).

// The handler is an Objective-C object (#229): unsafe code is allowed in
// this module only, to define it and to send the messages it needs.
#![allow(unsafe_code)]

use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString,
};

/// `kCoreEventClass`, the class of the open documents event.
const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
/// `kAEOpenDocuments`.
const OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
/// `keyDirectObject`, the files of the event.
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

/// Takes each file opened.
pub type Opened = Box<dyn Fn(PathBuf)>;

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Opener`
    // does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OldBrainFixOpener"]
    #[ivars = Opened]
    struct Opener;

    impl Opener {
        #[unsafe(method(willFinishLaunching:))]
        fn will_finish_launching(&self, _note: &NSNotification) {
            self.take_open_documents();
        }

        #[unsafe(method(openDocuments:withReply:))]
        fn open_documents(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            for path in paths(event) {
                (self.ivars())(path);
            }
        }
    }
);

impl Opener {
    fn new(main: MainThreadMarker, opened: Opened) -> Retained<Opener> {
        let this = main.alloc::<Opener>().set_ivars(opened);
        // SAFETY: NSObject's `init`, on an object just allocated.
        unsafe { msg_send![super(this), init] }
    }

    /// Makes this the handler of the open documents event, in place of
    /// AppKit's.
    fn take_open_documents(&self) {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        // SAFETY: this object answers `openDocuments:withReply:`, which
        // takes the event and its reply, and is never released
        // ([`install`]); the class and id are four-character codes.
        unsafe {
            let () = msg_send![
                &*manager,
                setEventHandler: self,
                andSelector: sel!(openDocuments:withReply:),
                forEventClass: CORE_EVENT_CLASS,
                andEventID: OPEN_DOCUMENTS
            ];
        }
    }
}

/// The files of an open documents event: a list of them, or one alone.
fn paths(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    // SAFETY: `paramDescriptorForKeyword:` takes a four-character code
    // and gives a descriptor or nil.
    let files: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
    let Some(files) = files else {
        return Vec::new();
    };
    let count = files.numberOfItems();
    let items: Vec<Retained<NSAppleEventDescriptor>> = if count > 0 {
        // Counted from 1.
        (1..=count)
            .filter_map(|i| files.descriptorAtIndex(i))
            .collect()
    } else {
        vec![files]
    };
    items
        .iter()
        .filter_map(|item| item.fileURLValue())
        .filter_map(|url| url.path())
        .map(|path| PathBuf::from(path.to_string()))
        .collect()
}

/// Hands each Markdown file the Finder opens with the app to `opened`.
/// Call on the main thread, before the window first runs.
pub fn install(opened: Opened) {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let opener = Opener::new(main, opened);
    let center = NSNotificationCenter::defaultCenter();
    let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
    // SAFETY: `opener` answers `willFinishLaunching:`, which takes the
    // notification, and is never released below.
    unsafe {
        center.addObserver_selector_name_object(
            &opener,
            sel!(willFinishLaunching:),
            Some(&*name),
            None,
        );
    }
    // Neither the notification centre nor AppKit keeps it; it lives as
    // long as the app.
    std::mem::forget(opener);
}
