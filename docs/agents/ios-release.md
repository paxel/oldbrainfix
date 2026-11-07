# iPhone release setup

What the user does once so the tag pipeline (`.github/workflows/release.yml`,
job `testflight`) can sign the iPhone app, its Share extension and its
Widgets extension and upload them to TestFlight. Until all nine secrets exist, that job skips with a
notice and the release goes on without the iPhone. The build and XCTest (`.github/workflows/ios.yml`)
need no secrets.

## 1. App IDs, App Group and app record

- [ ] developer.apple.com → Certificates, Identifiers & Profiles →
      Identifiers → **+** → **App Groups**. Description `Old Brain Fix`,
      identifier `group.io.github.paxel.oldbrainfix` (#241): the app and
      its Share extension hand shares over through it, and the app and
      its Widgets extension the Agenda widget's snapshot and its ticks
      (#242).
- [ ] Identifiers → **+** → App IDs → App. Description `Old Brain Fix`,
      Bundle ID **Explicit** `io.github.paxel.oldbrainfix`. Capabilities:
      **Time Sensitive Notifications** (#238) and **App Groups** (#241),
      both in `ios/Runner/Runner.entitlements`; after saving, the App
      Groups capability's **Configure** → tick
      `group.io.github.paxel.oldbrainfix`. Later tickets add theirs.
      AlarmKit, EventKit and Core Location need none, only their usage
      texts.
- [ ] Identifiers → **+** → App IDs → App. Description
      `Old Brain Fix Share`, Bundle ID **Explicit**
      `io.github.paxel.oldbrainfix.ShareExtension` (the Share extension,
      #241). Capabilities: **App Groups** only
      (`ios/ShareExtension/ShareExtension.entitlements`), configured with
      the same `group.io.github.paxel.oldbrainfix`.
- [ ] Identifiers → **+** → App IDs → App. Description
      `Old Brain Fix Widgets`, Bundle ID **Explicit**
      `io.github.paxel.oldbrainfix.Widgets` (the Agenda widget, the
      Capture widget and the Capture controls, #242). Capabilities:
      **App Groups** only (`ios/Widgets/Widgets.entitlements`),
      configured with the same `group.io.github.paxel.oldbrainfix`.
- [ ] appstoreconnect.apple.com → Apps → **+** → New App. Platform iOS,
      name `Old Brain Fix`, primary language English (U.S.), bundle id
      `io.github.paxel.oldbrainfix`, SKU `oldbrainfix`, full access.
- [ ] Copy the app's **Apple ID** (App Information → General) into
      `docs/agents/store-kit.md`.
- [ ] Note the **Team ID** (developer.apple.com → Membership details).

## 2. App Store Connect API key

- [ ] App Store Connect → Users and Access → Integrations → App Store
      Connect API → Team Keys → **+**. Name `oldbrainfix-ci`, access
      **App Manager**.
- [ ] Download the `AuthKey_<KEY ID>.p8` (only possible once). Note the
      **Key ID** and the **Issuer ID** shown above the list.

## 3. Distribution certificate

- [ ] On any Mac, Keychain Access → Certificate Assistant → Request a
      Certificate From a Certificate Authority → saved to disk. (Without a
      Mac: `openssl req -new -newkey rsa:2048 -nodes -keyout dist.key -out
      dist.csr -subj "/CN=Old Brain Fix"`.)
- [ ] developer.apple.com → Certificates → **+** → **Apple Distribution**,
      upload the request, download `distribution.cer`.
- [ ] Export it with its private key as `.p12` with a password: on a Mac,
      double-click the `.cer`, then in Keychain Access right-click the
      certificate → Export. Without a Mac:
      `openssl x509 -inform DER -in distribution.cer -out dist.pem` and
      `openssl pkcs12 -export -legacy -inkey dist.key -in dist.pem -out dist.p12`.

## 4. Provisioning profiles

- [ ] developer.apple.com → Profiles → **+** → Distribution → **App Store
      Connect**, App ID `io.github.paxel.oldbrainfix`, the certificate from
      step 3. Name `Old Brain Fix App Store`. Download the `.mobileprovision`.
- [ ] The same for the Share extension: **App Store Connect**, App ID
      `io.github.paxel.oldbrainfix.ShareExtension`, the same certificate.
      Name `Old Brain Fix Share App Store`. Download its `.mobileprovision`.
- [ ] The same for the Widgets extension: **App Store Connect**, App ID
      `io.github.paxel.oldbrainfix.Widgets`, the same certificate. Name
      `Old Brain Fix Widgets App Store`. Download its `.mobileprovision`.
- [ ] A profile made before an App ID got a capability lacks it: the App
      Group of #241 needs the app's profile made again and `IOS_PROFILE`
      replaced. Do the same whenever the certificate is renewed (then for
      all three profiles and both certificate secrets) or a capability is
      added.

## 5. Repository secrets

GitHub → paxel/oldbrainfix → Settings → Secrets and variables → Actions →
New repository secret. Base64 on one line: `base64 -w0 FILE` on Linux,
`base64 -i FILE` on a Mac.

| Secret                   | Value                                                      |
|--------------------------|------------------------------------------------------------|
| `ASC_KEY_ID`             | the Key ID from step 2                                     |
| `ASC_ISSUER_ID`          | the Issuer ID from step 2                                  |
| `ASC_KEY_P8`             | `base64 -w0 AuthKey_<KEY ID>.p8`                           |
| `IOS_DIST_CERT_P12`      | `base64 -w0 dist.p12`                                      |
| `IOS_DIST_CERT_PASSWORD` | the `.p12` export password                                 |
| `IOS_PROFILE`            | `base64 -w0 Old_Brain_Fix_App_Store.mobileprovision`       |
| `IOS_PROFILE_SHARE`      | `base64 -w0 Old_Brain_Fix_Share_App_Store.mobileprovision` |
| `IOS_PROFILE_WIDGETS`    | `base64 -w0 Old_Brain_Fix_Widgets_App_Store.mobileprovision` |
| `APPLE_TEAM_ID`          | the Team ID from step 1                                    |

Delete the local `.p8`, `.p12`, `dist.key` and the base64 output once the
secrets are saved, or keep them in a password manager.

## 6. How the job signs

The `testflight` job checks that each profile is for its bundle id
(`IOS_PROFILE` for `io.github.paxel.oldbrainfix`, `IOS_PROFILE_SHARE` for
`io.github.paxel.oldbrainfix.ShareExtension`, `IOS_PROFILE_WIDGETS` for
`io.github.paxel.oldbrainfix.Widgets`), installs all three, and appends
manual signing with the Apple Distribution certificate to three xcconfigs:
the Runner's `ios/Flutter/Release.xcconfig` with the app's profile and
each extension's own `ios/ShareExtension/Release.xcconfig` and
`ios/Widgets/Release.xcconfig` with its own. The export options name all
three bundle ids with their profiles, so `flutter build ipa` signs the
extensions inside the app. The extensions' version and build number come
from the same Flutter values as the app's, as App Store Connect requires.

## 7. After a tag

- [ ] The `iOS to TestFlight` job of the Release run uploads the build.
      A dispatch without publish signs it and uploads nothing. The upload
      is `xcrun altool --upload-app`; if a newer Xcode refuses it, the job
      fails at that step and needs `altool --upload-package` or Transporter.
- [ ] App Store Connect → Apps → Old Brain Fix → **TestFlight** → iOS
      Builds shows it after Apple's processing, usually 5 to 30 minutes;
      an email says when it is done.
- [ ] The first build asks the export-compliance question (Manage on the
      build). The app encrypts backups and sync; the answers, and the
      France decision that ends the question, are in
      `docs/agents/app-review-notes.md`.
- [ ] Add yourself under Internal Testing, then install through the
      TestFlight app on the iPhone.

## 8. Submitting to the App Store

- [ ] Once, before the first submission: the checklist "Before the first
      submission" in `docs/agents/app-review-notes.md` (App Privacy from
      `docs/agents/app-privacy.md`, age rating, pricing and
      availability, review contact, screenshots).
- [ ] For a release that goes to the App Store, write What's New into
      `fastlane/metadata/ios/<locale>/release_notes.txt` (not for the
      first version, which has none).
- [ ] Actions → Release → **Run workflow**: the tag, **publish** on and
      **submit_app_store** on. The `iOS to TestFlight` job uploads the
      build, then `fastlane deliver` uploads the listing and the App
      Review notes from `fastlane/metadata/ios/`, waits until Apple has
      processed that build and submits it for review. The submitted
      build is the one this run uploads, not an earlier TestFlight
      build, so its export compliance comes from
      `ITSAppUsesNonExemptEncryption` in `ios/Runner/Info.plist`; the
      step stops while that key is missing
      (`docs/agents/app-review-notes.md`). The screenshots stay as
      uploaded by hand.
- [ ] After approval, release the version by hand in App Store Connect:
      the submission sets no automatic release.
- A tag, or a dispatch without `submit_app_store`, never submits.
