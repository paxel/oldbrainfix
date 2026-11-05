import SwiftUI
import WidgetKit

/// The Widgets extension (#242): the Agenda widget, the Capture widget
/// for the home and lock screen, and the three Capture controls.
@main
struct OldBrainFixWidgets: WidgetBundle {
  var body: some Widget {
    AgendaWidget()
    CaptureWidget()
    VoiceCaptureControl()
    TextCaptureControl()
    PhotoCaptureControl()
  }
}
