import 'dart:async';

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

  /// The ids the questions came with, and those stopped (#317).
  final ids = <String>[];
  final stopped = <String>[];

  /// How far it is with a question; null for none, as 0.6.0 tells.
  AskProgress? Function(String id)? progressOf;

  /// While set, a question waits for it before its answer.
  Completer<void>? hold;

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
  Future<AskReply> ask(
    String question,
    List<AskExchange> earlier, {
    required String id,
  }) async {
    asked.add((question, earlier));
    ids.add(id);
    if (hold case final hold?) await hold.future;
    if (askTrouble case final trouble?) throw AskException(trouble);
    return answer(question);
  }

  @override
  Future<AskProgress?> progress(String id) async => progressOf?.call(id);

  @override
  Future<void> stop(String id) async => stopped.add(id);
}
