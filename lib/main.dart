import 'dart:io';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:path_provider/path_provider.dart';

import 'src/alarm_platform.dart';
import 'src/app.dart';
import 'src/drafts.dart';
import 'src/location.dart';
import 'src/photo_platform.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final dir = await getApplicationSupportDirectory();
  final store = Store.open('${dir.path}/oldbrainfix.db');
  runApp(
    OldBrainFixApp(
      store: store,
      drafts: FileDrafts(Directory('${dir.path}/drafts')),
      location: const AndroidLocation(),
      alarms: AndroidAlarms(),
      photos: AndroidPhotos(),
    ),
  );
}
