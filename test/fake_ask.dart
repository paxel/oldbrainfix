import 'package:oldbrainfix/src/ask_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

const fakeClaude = DesktopProvider(
  protocol: AskProtocol.anthropic,
  model: 'claude-opus-5-5',
  host: 'api.anthropic.com',
);

/// The user's desktop as the Ask page's tests and frames have it: the
/// provider it names, or [trouble] when it cannot be reached, and the
/// questions that reached it with their earlier exchanges.
class FakeAsking implements DesktopAsking {
  DesktopProvider shown;
  AskTrouble? trouble;

  /// What a question reaching it throws, when not null.
  AskTrouble? askTrouble;
  AskReply Function(String question) answer;
  final asked = <(String, List<AskExchange>)>[];

  @override
  String? computer = 'Desk';

  FakeAsking({
    this.shown = fakeClaude,
    this.trouble,
    AskReply Function(String question)? answer,
  }) : answer =
           answer ??
           ((question) => const AskReply.answered(
             DesktopAnswer(text: 'An answer.', notes: []),
           ));

  @override
  Future<DesktopProvider> provider() async {
    if (trouble case final trouble?) throw AskException(trouble);
    return shown;
  }

  @override
  Future<AskReply> ask(String question, List<AskExchange> earlier) async {
    asked.add((question, earlier));
    if (askTrouble case final trouble?) throw AskException(trouble);
    return answer(question);
  }
}
