import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'l10n.dart';
import 'photo_platform.dart';

/// One stored photo, by its hash: square-cropped at [size] for a
/// thumbnail, or at full width when [size] is null. A photo whose bytes
/// are not on this Device (yet) shows as a broken picture. Tapping runs
/// [onTap].
class NotePhoto extends StatefulWidget {
  final String hash;
  final double? size;
  final VoidCallback? onTap;

  const NotePhoto({super.key, required this.hash, this.size, this.onTap});

  @override
  State<NotePhoto> createState() => _NotePhotoState();
}

class _NotePhotoState extends State<NotePhoto> {
  Uint8List? _bytes;
  String? _read;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _load();
  }

  @override
  void didUpdateWidget(NotePhoto oldWidget) {
    super.didUpdateWidget(oldWidget);
    _load();
  }

  /// Reads the bytes once per hash, never while building.
  void _load() {
    if (_read == widget.hash) return;
    _read = widget.hash;
    _bytes = PhotoScope.bytesOf(context, widget.hash);
  }

  @override
  Widget build(BuildContext context) {
    final size = widget.size;
    final ratio = MediaQuery.devicePixelRatioOf(context);
    final bytes = _bytes;
    final broken = Icon(
      Icons.broken_image_outlined,
      size: size == null ? 48 : size / 2,
      semanticLabel: context.t.photoMissing,
    );
    final Widget image = bytes == null
        ? broken
        : Image.memory(
            bytes,
            fit: size == null ? BoxFit.fitWidth : BoxFit.cover,
            // A thumbnail is decoded small; it never holds the full photo.
            cacheWidth: size == null ? null : (size * ratio).round(),
            gaplessPlayback: true,
            semanticLabel: context.t.photo,
            errorBuilder: (_, _, _) => broken,
          );
    final framed = ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: size == null
          ? image
          : SizedBox(
              width: size,
              height: size,
              child: Center(child: image),
            ),
    );
    final onTap = widget.onTap;
    if (onTap == null) return framed;
    return GestureDetector(onTap: onTap, child: framed);
  }
}

/// A row of photo thumbnails, scrolling sideways when it outgrows the
/// width.
class PhotoStrip extends StatelessWidget {
  final List<String> hashes;
  final double size;

  /// Runs with the photo tapped; nothing happens on a tap when null.
  final ValueChanged<String>? onTap;

  const PhotoStrip({
    super.key,
    required this.hashes,
    this.size = 72,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) => SizedBox(
    height: size,
    child: ListView.separated(
      scrollDirection: Axis.horizontal,
      itemCount: hashes.length,
      separatorBuilder: (_, _) => const SizedBox(width: 8),
      itemBuilder: (context, i) {
        final hash = hashes[i];
        final onTap = this.onTap;
        return NotePhoto(
          key: ValueKey(hash),
          hash: hash,
          size: size,
          onTap: onTap == null ? null : () => onTap(hash),
        );
      },
    ),
  );
}

/// Opens the photo [hash] full screen.
Future<void> viewPhoto(BuildContext context, String hash) => Navigator.of(
  context,
).push(MaterialPageRoute<void>(builder: (_) => PhotoViewerScreen(hash: hash)));

/// One photo over the whole screen, to zoom and pan.
class PhotoViewerScreen extends StatelessWidget {
  final String hash;

  const PhotoViewerScreen({super.key, required this.hash});

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: Text(context.t.photo)),
    body: SafeArea(
      child: InteractiveViewer(
        maxScale: 6,
        child: Center(child: NotePhoto(hash: hash)),
      ),
    ),
  );
}
