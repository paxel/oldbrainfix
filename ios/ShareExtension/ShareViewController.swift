import UIKit
import UniformTypeIdentifiers

/// "Old Brain Fix" in the share sheet (#241): keeps what another app
/// shares, its text, links, pictures, files of the app's own types and
/// any other files as Attachments (#267),
/// in the App Group's inbox (ShareInbox.swift) and says so. A share
/// extension cannot open its app, so the app takes it the next time it
/// opens or comes back: text and pictures into Capture, a bundle, Sent
/// Item, Profile card or Pack as "Open with" opens it. Pictures are
/// copied as they are; the app makes them into photos, as decoding here
/// would cost the extension's small memory.
final class ShareViewController: UIViewController {
  private let message = UILabel()
  private let done = UIButton(type: .system)
  private var started = false

  override func viewDidLoad() {
    super.viewDidLoad()
    view.backgroundColor = UIColor.black.withAlphaComponent(0.3)
    let card = UIView()
    card.backgroundColor = .systemBackground
    card.layer.cornerRadius = 16
    card.translatesAutoresizingMaskIntoConstraints = false
    message.numberOfLines = 0
    message.textAlignment = .center
    message.font = .preferredFont(forTextStyle: .body)
    message.adjustsFontForContentSizeCategory = true
    message.text = Self.text("keeping")
    done.setTitle(Self.text("ok"), for: .normal)
    done.titleLabel?.font = .preferredFont(forTextStyle: .headline)
    done.isEnabled = false
    done.addTarget(self, action: #selector(close), for: .touchUpInside)
    let stack = UIStackView(arrangedSubviews: [message, done])
    stack.axis = .vertical
    stack.spacing = 16
    stack.translatesAutoresizingMaskIntoConstraints = false
    card.addSubview(stack)
    view.addSubview(card)
    NSLayoutConstraint.activate([
      card.centerYAnchor.constraint(equalTo: view.centerYAnchor),
      card.leadingAnchor.constraint(equalTo: view.layoutMarginsGuide.leadingAnchor, constant: 8),
      card.trailingAnchor.constraint(equalTo: view.layoutMarginsGuide.trailingAnchor, constant: -8),
      stack.topAnchor.constraint(equalTo: card.topAnchor, constant: 24),
      stack.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -16),
      stack.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 20),
      stack.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -20),
    ])
  }

  override func viewDidAppear(_ animated: Bool) {
    super.viewDidAppear(animated)
    guard !started else { return }
    started = true
    let items = (extensionContext?.inputItems as? [NSExtensionItem]) ?? []
    Self.keep(items) { [weak self] kept in
      guard let self else { return }
      self.message.text = Self.text(kept ? "kept" : "notKept")
      self.done.isEnabled = true
      self.kept = kept
      UIAccessibility.post(notification: .announcement, argument: self.message.text)
    }
  }

  private var kept = false

  @objc private func close() {
    if kept {
      extensionContext?.completeRequest(returningItems: nil)
    } else {
      extensionContext?.cancelRequest(
        withError: NSError(domain: "io.github.paxel.oldbrainfix.share", code: 1))
    }
  }

  private static func text(_ key: String) -> String {
    NSLocalizedString(key, bundle: Bundle(for: ShareViewController.self), comment: "")
  }

  /// Copies [items] into the inbox, then tells [done] on the main thread
  /// whether anything was kept.
  private static func keep(_ items: [NSExtensionItem], done: @escaping (Bool) -> Void) {
    guard let inbox = ShareInbox.directory() else {
      done(false)
      return
    }
    let work = DispatchQueue(label: "io.github.paxel.oldbrainfix.share")
    work.async {
      let folder = ShareInbox.write(into: inbox) { folder in
        try collect(items, into: folder)
      }
      DispatchQueue.main.async { done(folder != nil) }
    }
  }

  /// Loads every attachment of [items] the app takes, files copied into
  /// [folder]; the manifest of it all. Runs off the main thread and
  /// waits for each provider in turn, in the order shared.
  private static func collect(_ items: [NSExtensionItem], into folder: URL) throws
    -> ShareManifest
  {
    var manifest = ShareManifest()
    for item in items {
      if let title = item.attributedTitle?.string, !title.isEmpty, manifest.subject == nil {
        manifest.subject = title
      }
      if let content = item.attributedContentText?.string { manifest.texts.append(content) }
      for provider in item.attachments ?? [] {
        guard
          let match = ShareTypes.kind(
            of: provider.registeredTypeIdentifiers, name: provider.suggestedName)
        else { continue }
        let (kind, type) = match
        switch kind {
        case .file(let ext):
          if let name = copyFile(provider, type: type, into: folder, ext: ext) {
            manifest.files.append(name)
          }
        case .photo:
          if let name = copyPicture(provider, type: type, into: folder) {
            manifest.photos.append(name)
          }
        case .link:
          if let url = load(provider, type: type) as? URL { manifest.texts.append(url.absoluteString) }
        case .text:
          if let text = loadText(provider, type: type) { manifest.texts.append(text) }
        case .attachment:
          let ext = UTType(type)?.preferredFilenameExtension ?? "bin"
          if let file = copyFile(provider, type: type, into: folder, ext: ext) {
            let name = provider.suggestedName.map { suggested in
              (suggested as NSString).pathExtension.isEmpty ? "\(suggested).\(ext)" : suggested
            }
            manifest.attachments = (manifest.attachments ?? []) + [
              SharedAttachment(
                file: file, name: name ?? file,
                type: UTType(type)?.preferredMIMEType ?? "")
            ]
          }
        }
      }
    }
    return manifest
  }

  /// The provider's item as [type], waited for.
  private static func load(_ provider: NSItemProvider, type: String) -> NSSecureCoding? {
    let wait = DispatchSemaphore(value: 0)
    var loaded: NSSecureCoding?
    provider.loadItem(forTypeIdentifier: type, options: nil) { item, _ in
      loaded = item
      wait.signal()
    }
    wait.wait()
    return loaded
  }

  private static func loadText(_ provider: NSItemProvider, type: String) -> String? {
    switch load(provider, type: type) {
    case let text as String: return text
    case let text as NSAttributedString: return text.string
    case let data as Data: return String(data: data, encoding: .utf8)
    case let url as URL where url.isFileURL: return try? String(contentsOf: url, encoding: .utf8)
    default: return nil
    }
  }

  /// Copies the provider's file as [type] into [folder] as a new name
  /// with [ext]; that name, or nil when there was none to copy.
  private static func copyFile(
    _ provider: NSItemProvider, type: String, into folder: URL, ext: String
  ) -> String? {
    let name = "\(UUID().uuidString).\(ext)"
    let target = folder.appendingPathComponent(name)
    let wait = DispatchSemaphore(value: 0)
    var copied = false
    // The file goes away once the handler returns: copied inside it.
    provider.loadFileRepresentation(forTypeIdentifier: type) { url, _ in
      if let url { copied = (try? FileManager.default.copyItem(at: url, to: target)) != nil }
      wait.signal()
    }
    wait.wait()
    return copied ? name : nil
  }

  /// Copies the provider's picture into [folder] as it is: from its
  /// file, else from its bytes, else a picture handed over in memory as
  /// a PNG.
  private static func copyPicture(_ provider: NSItemProvider, type: String, into folder: URL)
    -> String?
  {
    let ext = UTType(type)?.preferredFilenameExtension ?? "img"
    if let name = copyFile(provider, type: type, into: folder, ext: ext) { return name }
    let data: Data?
    var named = ext
    switch load(provider, type: type) {
    case let bytes as Data: data = bytes
    case let picture as UIImage:
      data = picture.pngData()
      named = "png"
    case let url as URL where url.isFileURL: data = try? Data(contentsOf: url)
    default: data = nil
    }
    guard let data, !data.isEmpty else { return nil }
    let name = "\(UUID().uuidString).\(named)"
    return (try? data.write(to: folder.appendingPathComponent(name))) != nil ? name : nil
  }
}
