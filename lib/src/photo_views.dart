import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'l10n.dart';
import 'photo_platform.dart';

/// One stored photo, by its hash: square-cropped at [size] for a
/// thumbnail, or at full width when [size] is null. With [maxHeight] it
/// keeps its own proportions at full width, no taller than that, and is
/// decoded no bigger than it shows, so a photo of 12 × 34,000 pixels
/// cannot break a card; otherwise it is decoded at most [largest] pixels
/// either way. A photo whose bytes are not on this Device (yet)
/// shows as a broken picture. Tapping runs [onTap].
class NotePhoto extends StatefulWidget {
  final String hash;
  final double? size;
  final double? maxHeight;
  final VoidCallback? onTap;

  /// How low a photo at full width may get, so a very wide one can still
  /// be tapped.
  static const minHeight = 48.0;

  /// The most pixels a photo is decoded at either way, full screen and
  /// in the editor: a 12-megapixel photo keeps its detail for the zoom,
  /// and no photo outgrows what a graphics texture holds.
  static const largest = 4096;

  const NotePhoto({
    super.key,
    required this.hash,
    this.size,
    this.maxHeight,
    this.onTap,
  });

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
    final maxHeight = widget.maxHeight;
    final Widget image = bytes == null
        ? broken
        : size == null && maxHeight != null
        ? LayoutBuilder(
            builder: (context, box) => ConstrainedBox(
              // Full width; as high as its proportions say, within bounds.
              constraints: BoxConstraints.tightFor(
                width: box.maxWidth,
              ).copyWith(minHeight: NotePhoto.minHeight, maxHeight: maxHeight),
              child: Image(
                image: BoundedPhoto(
                  widget.hash,
                  bytes,
                  maxWidth: (box.maxWidth * ratio).ceil(),
                  maxHeight: (maxHeight * ratio).ceil(),
                ),
                fit: BoxFit.contain,
                gaplessPlayback: true,
                semanticLabel: context.t.photo,
                errorBuilder: (_, _, _) => broken,
              ),
            ),
          )
        : size == null
        // Never more than a texture holds, however extreme the photo.
        ? Image(
            image: BoundedPhoto(
              widget.hash,
              bytes,
              maxWidth: NotePhoto.largest,
              maxHeight: NotePhoto.largest,
            ),
            fit: BoxFit.fitWidth,
            gaplessPlayback: true,
            semanticLabel: context.t.photo,
            errorBuilder: (_, _, _) => broken,
          )
        : Image.memory(
            bytes,
            fit: BoxFit.cover,
            // A thumbnail is decoded small; it never holds the full photo.
            cacheWidth: (size * ratio).round(),
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

/// A stored photo decoded to fit within [maxWidth] × [maxHeight]
/// pixels in its own proportions, never larger than it is and at least
/// one pixel each way: unlike [ResizeImage], an extreme aspect ratio
/// such as 12 × 34,000 still gives a picture to draw.
@immutable
class BoundedPhoto extends ImageProvider<BoundedPhoto> {
  /// The photo's hash, which names its bytes.
  final String hash;
  final Uint8List bytes;
  final int maxWidth;
  final int maxHeight;

  const BoundedPhoto(
    this.hash,
    this.bytes, {
    required this.maxWidth,
    required this.maxHeight,
  });

  /// The size [width] × [height] decodes at within the bounds.
  static ui.TargetImageSize fit(
    int width,
    int height, {
    required int maxWidth,
    required int maxHeight,
  }) {
    final scale = math.min(1.0, math.min(maxWidth / width, maxHeight / height));
    return ui.TargetImageSize(
      width: math.max(1, (width * scale).round()),
      height: math.max(1, (height * scale).round()),
    );
  }

  @override
  Future<BoundedPhoto> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(
    BoundedPhoto key,
    ImageDecoderCallback decode,
  ) => MultiFrameImageStreamCompleter(codec: _decode(decode), scale: 1);

  Future<ui.Codec> _decode(ImageDecoderCallback decode) async => decode(
    await ui.ImmutableBuffer.fromUint8List(bytes),
    getTargetSize: (width, height) =>
        fit(width, height, maxWidth: maxWidth, maxHeight: maxHeight),
  );

  @override
  bool operator ==(Object other) =>
      other is BoundedPhoto &&
      other.hash == hash &&
      other.maxWidth == maxWidth &&
      other.maxHeight == maxHeight;

  @override
  int get hashCode => Object.hash(hash, maxWidth, maxHeight);
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
