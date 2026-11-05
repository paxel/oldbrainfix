# App Review notes

What App Store Connect asks in **App Review Information** for each
version (#245), and the export-compliance answer for each build.

## Notes

App Store Connect → Apps → Old Brain Fix → the version → App Review
Information → **Notes** (up to 4000 bytes). The release job's
`app-store-review` path uploads the same text from
`fastlane/metadata/ios/review_information/notes.txt`;
`tool/appstore_listing.py` fails when the two differ, so change both
together.

```text
Old Brain Fix is a note and reminder app. It has no account, no login and no server, so no demo account is needed: every feature works right after the first start. All data stays on the iPhone and on Devices the user pairs with it (other phones and the user's computer, which runs the desktop app).

How to test
Steps 1 to 3 take about a minute; step 4 takes about ten.
1. Start the app, tap "New to Old Brain Fix", pick Simple and tap Start. Tap "Not now" on the speech model offer. On "Set up alarms", tap Allow on each step (Notifications, then Alarms) and Next, and Done on the last.
2. Tap the round + at the bottom right of the Agenda, tap Text, type a thought and tap 1 h. The Note is saved in Unsorted.
3. Tap the alarm clock on the Agenda ("New Routine or Reminder"), type what is to be done, tap Once, add a time two minutes ahead and tap Save. A time-sensitive notification comes at that minute, with Done and Snooze.
4. To see the AlarmKit alarm: in the same editor open Re-nag and pick 5 min under "This one", then open Escalation, switch on "Escalate to an iPhone alarm" and pick 1. Leave the notifications alone: the first comes at the set time, the Re-nag 5 minutes later, and about 10 minutes after the set time it rings as an iPhone alarm with Stop and Snooze.

AlarmKit
The app uses AlarmKit only for Routines and Reminders the user has set up, and only when the user has switched on Escalation for that one: after the set number of ignored notifications it rings as an alarm, through silent mode and Focus, so a reminder the user keeps missing is not missed again. Stop counts as Done; Snooze rings it again in 10 minutes. Without Escalation every Routine and Reminder is a time-sensitive notification.

Local network
The app syncs with the user's own paired Devices on the same Wi-Fi, directly and encrypted with TLS, without the internet. It announces itself by Bonjour (_oldbrainfix._tcp) only while it is open, carrying a random device id and no name. Pairing is done once by scanning or typing a code another Device shows. The app works fully without a second Device; the review does not need one.

Calendars (full access)
Only when the user switches it on: the Agenda shows the appointments of the calendars the user picks, a Note records the event running when it was captured, and the user's Routines and Reminders can be written into one calendar the user chooses. Showing appointments needs to read events, which iOS grants only with full access. Nothing read from the calendars leaves the iPhone.

Share extension
"Old Brain Fix" in the share sheet takes text, links, pictures and the app's own files. It keeps them in the App Group shared with the app and shows a short card that they were kept; it does not open the app. They appear in Capture the next time the app is opened.

Widgets and Control Center
An Agenda widget (medium and large), a Capture widget (home screen and lock screen) and three Control Center controls (Voice note, Text note, Photo note) open the app on the matching page.

Other permissions, each asked only when the feature is used: microphone for voice Notes, camera for photo Notes and pair codes, adding photos to Photos when the user taps Save, location while in use for where a Note was captured, Face ID for the optional App Lock, notifications.

The network is used only for the local sync and for downloading the speech model (148 MB, from the app's own GitHub release) when the user asks for it. There is no analytics, no advertising and no tracking.

The app is free and open source (Apache-2.0 or MIT): https://github.com/paxel/oldbrainfix
```

## The rest of App Review Information

- **Sign-in required**: off. The app has no account and no login, so
  there is no demo account to give.
- **Contact information** (first and last name, phone, email): the
  user's own, typed in App Store Connect once. It stays out of the repo;
  the submission path never sends it and leaves what is there.
- **Attachment**: none needed.

## Before the first submission

Fields App Store Connect needs once and the submission path does not
set; each blocks **Add for Review** while empty:

- [ ] App Privacy: the answers in `docs/agents/app-privacy.md`, then
      **Publish**.
- [ ] Age Rating: the questionnaire. The app has no web browsing, no
      purchases, no ads and nothing shown to strangers; Notes go only to
      people the user invites to a Shared Area or sends a file to, which
      is worth a look at the questions on user-to-user content.
- [ ] Pricing and Availability: Free, and the countries (see France
      under Export compliance).
- [ ] App Review contact information, as above.
- [ ] The screenshots, uploaded by hand from
      `fastlane/metadata/ios/<locale>/screenshots/iphone-6.9/`
      (`docs/agents/store-kit.md`).
- [ ] `ITSAppUsesNonExemptEncryption` set in `ios/Runner/Info.plist`
      after the France decision (below).

A known review risk outside these texts: the About page links to Ko-fi
("Buy me a coffee"). Guideline 3.1.1 asks for in-app purchase where an
app takes tips for its developer, and App Review may ask for the link to
go on the iPhone. Decide before the first submission; no code is changed
for it here.

## Export compliance

Every build asks whether the app uses encryption until the Info.plist
answers it. The app does:

- XChaCha20-Poly1305 for sync bundles, shared-folder files, Sent Items
  and Backups, Argon2id for the Backup passphrase, Ed25519 signatures,
  all in pure Dart (`cryptography`, `ed25519_edwards`, ADR 0003, 0012);
- TLS for the local-network sync with an ECDSA P-256 self-signed
  certificate (`pointycastle`), through Dart's `SecureSocket`, which is
  BoringSSL inside the Flutter engine, not Apple's TLS;
- SHA-256 from Apple's CryptoKit, only to check the speech model's
  download.

All of them are standard, published algorithms, none proprietary, and
none of the encryption but the SHA-256 is Apple's.

What Apple says, read 2026-11-05:

- [Complying with Encryption Export Regulations](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations):
  "Set the value to NO if your app—including any third-party libraries
  it links against—doesn't use encryption, or if it only uses forms of
  encryption that are exempt from export compliance documentation
  requirements. Otherwise, set it to YES." And: "Typically, the use of
  encryption that's built into the operating system … is exempt from
  export documentation upload requirements, whereas the use of
  proprietary encryption is not."
- [Export compliance documentation for encryption](https://developer.apple.com/help/app-store-connect/reference/app-information/export-compliance-documentation-for-encryption):
  "Your app uses an industry standard algorithm, not provided within the
  Apple operating system" → "Upload your French encryption declaration
  in App Store Connect", with the footnote "French encryption
  declaration form is only required if you're distributing your app on
  the App Store in France." Encryption "limited to that within the Apple
  operating system" needs no documentation; proprietary algorithms need
  a CCATS as well.
- [Overview of export compliance](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance):
  "The main items of control for France are Secure Storage, Secure
  Communications, and Security Anti-Virus applications." Encrypted
  Backups and an encrypted sync are both.

The United States side: the source is public on GitHub and uses only
standard cryptography. Under EAR § 742.15(b) publicly available
encryption source code classified 5D002 "is not subject to the EAR",
and the notification to BIS and the NSA is required only for
"non-standard cryptography"; under the note to § 734.3(b)(2) and (b)(3)
its object code "is not subject to the EAR when the corresponding source
code meets the criteria" of § 742.15(b)
([742.15](https://www.law.cornell.edu/cfr/text/15/742.15),
[734.3](https://www.law.cornell.edu/cfr/text/15/734.3)). No CCATS, no
notification and no year-end self-classification report follow from
that. This is a reading of the regulations, not legal advice.

### Decision for the user

The only open question is France, so `ITSAppUsesNonExemptEncryption` is
not set in `ios/Runner/Info.plist` yet; App Store Connect asks on each
build until it is. Either way is one line there:

| Choice                                      | Info.plist                                                                                                               | In App Store Connect                                                                                                                                                                                                                     |
|---------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| France left out (recommended for the first) | `ITSAppUsesNonExemptEncryption` = NO                                                                                     | Nothing to upload. On the build's question: uses encryption, standard algorithms instead of or in addition to Apple's, not in France.                                                                                                    |
| France included                             | `ITSAppUsesNonExemptEncryption` = YES and `ITSEncryptionExportComplianceCode` = the code Apple gives for the declaration | App Information → App Encryption Documentation → **+**, answer the questions, upload the French declaration (ANSSI, the declaration of supply of a means of cryptology); Apple reviews it in about two business days and shows the code. |

Recommended: leave France out for the first release and set NO with
it; that needs no filing and can change later by filing the declaration
and switching to the second row. NO is only true while France is out of
Availability, so the two change together.

Until one of them is chosen, answer each TestFlight build's question by
hand (TestFlight → the build → **Manage**, `docs/agents/ios-release.md`
step 7): it uses encryption; the algorithms are standard ones, not only
Apple's; and whether the app is available in France.

The submission path in the release job cannot wait for a hand answer:
it submits the build the same run has just uploaded. So it needs the
Info.plist key, and stops when the key is missing, after the TestFlight
upload and before anything goes to the App Store listing. Set the key,
with its row's precondition met, before the first `submit_app_store`
run.
