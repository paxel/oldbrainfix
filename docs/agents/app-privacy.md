# App Privacy

The answers for App Store Connect → Apps → Old Brain Fix → **App
Privacy** (#245), from the privacy policy
(`site/en/reference/privacy-policy.html`) and the code. Fill it in once
and publish; change it only when the privacy policy changes what leaves
the iPhone.

## Apple's definitions

From [App privacy details on the App Store](https://developer.apple.com/app-store/app-privacy-details/),
read 2026-11-05:

- **Collect**: "'Collect' refers to transmitting data off the device in
  a way that allows you and/or your third-party partners to access it
  for a period longer than what is necessary to service the transmitted
  request in real time."
- **On the device**: "Data that is processed only on device is not
  'collected' and does not need to be disclosed in your answers. If you
  derive anything from that data and send it off device, the resulting
  data should be considered separately."
- **Tracking**: "'Tracking' refers to linking data collected from your
  app about a particular end-user or device, such as a user ID, device
  ID, or profile, with Third-Party Data for targeted advertising or
  advertising measurement purposes, or sharing data collected from your
  app about a particular end-user or device with a data broker."
- **Optional disclosure**: data types that meet all of these are
  optional to disclose: not used for tracking; not used for Third-Party
  Advertising, the developer's Advertising or Marketing, or Other
  Purposes; "Collection of the data occurs only in infrequent cases that
  are not part of your app's primary functionality, and which are
  optional for the user"; and "The data is provided by the user in your
  app's interface, it is clear to the user what data is collected, the
  user's name or account name is prominently displayed in the submission
  form alongside the other data elements being submitted, and the user
  affirmatively chooses to provide the data for collection each time."
  Apple's example: "data collected in optional feedback forms or
  customer service requests that are unrelated to the primary purpose of
  the app and meet the other criteria above."

"You" is the developer; "third-party partners" are those the developer
works with. The user's own Devices, the people the user shares an Area
with, and the services the user picks (a mail app, iCloud Drive, a
calendar account) are neither.

## The answer

**Do you or your third-party partners collect data from this app?**
**No, we do not collect data from this app.**

The label then reads **Data Not Collected**. App Store Connect asks
nothing more: no data types, no linking, no tracking. No App Tracking
Transparency prompt is needed either, because the app does not track.

## Why each data type is not collected

Each line names what the app does with the data and why that is not
"collect" in Apple's sense.

| Apple's data type                                  | What the app does                                                                                                                                         | Why it is not collected                                                                                              |
|----------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| Contact Info (name, email, phone, address)         | The Profile's name, typed by the user, travels with Notes the user shares, to the user's own Devices and to the people of a Shared Area.                  | It never reaches the developer or a partner; it goes device to device, as the user chooses.                          |
| Health and Fitness                                 | None. A Tally or Routine the user names after medication stays a Note.                                                                                    | Processed only on the device and the user's own Devices.                                                             |
| Financial Info                                     | None. No purchases in the app.                                                                                                                            | Not handled.                                                                                                         |
| Location (precise, coarse)                         | Read at Capture, while the app is open and only once switched on, matched against the Named Places on the iPhone, kept with the Note.                     | Stored on the device and synced only to the user's own Devices and Shared Areas; the developer never receives it.    |
| Sensitive Info                                     | None asked for.                                                                                                                                           | Not handled.                                                                                                         |
| Contacts                                           | Not read. The app has no contacts access.                                                                                                                 | Not handled.                                                                                                         |
| User Content: emails or text messages              | None. No messaging between users inside the app.                                                                                                          | Not handled.                                                                                                         |
| User Content: photos or videos                     | Photos taken or picked for Notes; Photos access is add-only.                                                                                              | Stored on the device and the user's Devices; never sent to the developer.                                            |
| User Content: audio data                           | Voice Notes, transcribed on the device by Whisper.                                                                                                        | Processed only on the device; the recordings go only to the user's Devices.                                          |
| User Content: gameplay content                     | None.                                                                                                                                                     | Not handled.                                                                                                         |
| User Content: customer support                     | Send feedback and crash reports are written by the user and leave only through the mail app or share target the user picks, showing all of it beforehand. | The app transmits nothing itself; the user sends a mail. Even read as collection, it is Apple's named optional case. |
| User Content: other                                | Notes, Routines, Reminders, history, Calendar events read for the Agenda (never stored), the calendar event at Capture.                                   | On the device and the user's Devices only.                                                                           |
| Browsing History, Search History                   | Search runs over the user's own Notes on the device.                                                                                                      | Processed only on the device.                                                                                        |
| Identifiers (user ID, device ID)                   | A random device id and the Profile's code, used for pairing, sync and the Bonjour announcement on the local network.                                      | They go only to the user's paired Devices and Shared Areas; there is no server and no advertising identifier.        |
| Purchases                                          | None.                                                                                                                                                     | Not handled.                                                                                                         |
| Usage Data (product interaction, advertising data) | None. No analytics, no advertising.                                                                                                                       | Not handled.                                                                                                         |
| Diagnostics (crash data, performance data)         | The last crash report is kept on the iPhone and goes only in a mail the user sends (see customer support).                                                | No automatic crash reporting; nothing is transmitted by the app.                                                     |
| Surroundings, Body                                 | None. Face ID answers only whether it matched; the face never reaches the app.                                                                            | Not handled.                                                                                                         |
| Other Data                                         | The connected Bluetooth audio route's name at Capture, once switched on.                                                                                  | Kept with the Note on the device and the user's Devices.                                                             |

## Network traffic that is not collection

- **The speech model download**: a GET of a fixed file from the app's
  own GitHub release, only when the user asks. GitHub sees the IP address
  as any web host does, only for serving that request; the developer
  receives nothing, and nothing about the user is sent. This is
  "servicing the transmitted request in real time".
- **The local-network sync and Bonjour announcement**: device to device
  on the user's network, never through the developer.
- **The shared folder**: files the app writes, end-to-end encrypted with
  the Area's key, into a folder the user picks; the carrier (iCloud
  Drive, Möbius Sync) is the user's choice and cannot read them.
- **Links on the About page** (the site, GitHub, Ko-fi, mail) open in
  the browser or mail app; the app sends nothing with them.
- **The desktop's Ask panel** (#267, ADR 0051) is not the iPhone app's:
  the desktop sends a question, and what its model finds and reads in
  the Notes, to the AI provider the user entered with their own key,
  never to the developer, and nothing Hidden from AI.
- **The phone's Ask through the user's desktop** (#267, ADR 0003, ADR
  0051): the iPhone sends the question, and the earlier questions and
  answers of the same conversation, over the paired connection to the
  user's own desktop, as a sync does (TLS, pinned certificate, the
  phone's proof), on the local network or through a VPN the user runs;
  the desktop asks its provider and sends back the answer's text and the
  ids of the Notes it links. The iPhone itself talks to no AI provider
  and holds no key; it learns the provider's protocol, model and host
  from the desktop and keeps, as a local setting, which ones the user
  allowed. Neither the developer nor a party the developer works with
  receives anything: the user's desktop is the user's own Device, and
  the provider is the user's choice with the user's own account, so
  this is not "collect", and the label stays **Data Not Collected**.
- **Guideline 5.1.2(i)** applies all the same, as the question reaches
  a third-party AI through the desktop: before the first question to a
  provider, the Ask page names the user's computer, the provider, its
  model and host, and what is sent (the question and earlier exchanges;
  the names of Areas, Topics and tags; Routines and Reminders with their
  check-offs when listed; titles and excerpts of the Notes found; the
  Notes read in full with their Sections, Transcripts, Photo Texts,
  Attachment Texts and Capture Context; nothing Hidden from AI), and
  asks with **Allow** and **Not now**. Nothing is sent before Allow; a
  provider, host or model the user did not allow is asked about again
  before the next question goes out (`askAllowed`, `allowAsking` in
  `device_settings.dart`). The privacy policy says the same under
  "Asking from the phone".

## Google Play: Data safety

Re-checked for #267 against the same facts, not against Play's
definitions read anew: Play's answers stay **No data collected** and
**No data shared**. The Android app sends a question only to the user's
own paired desktop, end to end over TLS with the pinned certificate,
never to the developer or a party the developer works with; the desktop,
not the Android app, sends it on to the provider the user set up with
their own key. Nothing new is stored off the device by the app. Read
Play's definitions of "collection" and "sharing" in the Play Console
before the next submission and note them here as above for Apple.

## Privacy Policy URL

Per localization, the URLs in `fastlane/metadata/ios/<locale>/privacy_url.txt`:

- English: https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html
- German: https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html

## When to look again

Answer again whenever a change makes the app send data off the iPhone
to the developer or a party the developer works with: a server, crash
reporting, analytics, an SDK, or a purchase. Then Apple's questions on
data types, linking and tracking apply, and the privacy policy changes
first.
