import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:path_provider/path_provider.dart';

import 'src/app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final dir = await getApplicationSupportDirectory();
  final store = Store.open('${dir.path}/oldbrainfix.db');
  runApp(OldBrainFixApp(store: store));
}
