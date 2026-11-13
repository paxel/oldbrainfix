import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'attachment_views.dart';
import 'failure_dialog.dart';
import 'file_platform.dart';
import 'l10n.dart';
import 'page_bar.dart';

/// How many pixels wide the [page], of that size in points, is drawn to
/// fill a [screen] of that logical size at [ratio] device pixels per
/// logical one, zoomed [scale] times (#286): as wide as it shows, at most
/// [maxRenderSide] pixels a side and [maxRenderPixels] in all.
int pdfRenderWidth(Size page, Size screen, double ratio, double scale) {
  final aspect = page.width / page.height;
  final shown = math.min(screen.width, screen.height * aspect);
  final width = shown * ratio * scale;
  final height = width / aspect;
  final shrink = [
    1.0,
    maxRenderSide / math.max(width, height),
    math.sqrt(maxRenderPixels / (width * height)),
  ].reduce(math.min);
  return math.max(1, (width * shrink).floor());
}

/// A page drawn: the [width] it was asked for, and its picture.
typedef _Drawn = ({int width, Uint8List png});

/// An attached PDF over the whole screen (#286), as a photo shows: its
/// pages swiped sideways, each zoomed and panned with two fingers, the
/// page shown and their count at the top. The system's own renderer
/// ([FilePlatform.pdfRender]) draws only the page shown and the one on
/// either side, as large as they show, the page shown anew when it is
/// zoomed. The Attachment Text button shows the PDF's [text] instead of
/// its pages; the ⋮ opens it in another app. The copy at [path] goes
/// when the viewer closes, unless another app was handed it.
class PdfViewerScreen extends StatefulWidget {
  final Attachment file;
  final String path;
  final List<Size> pages;
  final String? text;

  const PdfViewerScreen({
    super.key,
    required this.file,
    required this.path,
    required this.pages,
    this.text,
  });

  @override
  State<PdfViewerScreen> createState() => _PdfViewerScreenState();
}

class _PdfViewerScreenState extends State<PdfViewerScreen> {
  /// How many viewers show the copy at each path: a file opened again
  /// before its viewer is gone is written to the same path, and only the
  /// last viewer closed takes it away.
  static final _viewers = <String, int>{};

  final _pager = PageController();
  final _zoom = TransformationController();

  /// The pages drawn, by their index, never more than three.
  final _drawn = <int, _Drawn>{};

  /// The pages the renderer could not draw, not asked for again.
  final _failed = <int>{};

  /// For a page drawn whose sharper drawing failed, as one too large for
  /// the memory left: the width that failed, not asked for again.
  final _ceiling = <int, int>{};

  FilePlatform _files = const NoFiles();
  Size _screen = Size.zero;
  double _ratio = 1;
  int _page = 0;

  /// How far the page shown is zoomed, as its last gesture left it.
  double _scale = 1;
  bool _drawing = false;
  bool _showText = false;

  /// Whether another app was handed the copy, which then stays.
  bool _handed = false;

  bool get _zoomed => _scale > 1;

  /// Whether there is a text to show, or word that it follows.
  bool get _hasText => switch (widget.text) {
    final text? => text.isNotEmpty,
    null => widget.file.computer && widget.file.kind.hasText,
  };

  @override
  void initState() {
    super.initState();
    _viewers.update(widget.path, (n) => n + 1, ifAbsent: () => 1);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _files = FileScope.of(context);
    _screen = MediaQuery.sizeOf(context);
    _ratio = MediaQuery.devicePixelRatioOf(context);
    unawaited(_draw());
  }

  @override
  void dispose() {
    _pager.dispose();
    _zoom.dispose();
    for (final drawn in _drawn.values) {
      unawaited(MemoryImage(drawn.png).evict());
    }
    final left = (_viewers[widget.path] ?? 1) - 1;
    if (left > 0) {
      _viewers[widget.path] = left;
    } else {
      _viewers.remove(widget.path);
    }
    if (!_handed && left <= 0) {
      try {
        File(widget.path).deleteSync();
      } on FileSystemException {
        // Gone already.
      }
    }
    super.dispose();
  }

  /// The width the page [index] is wanted at now.
  int _wanted(int index) => pdfRenderWidth(
    widget.pages[index],
    _screen,
    _ratio,
    index == _page ? _scale : 1,
  );

  /// The next page to draw: the one shown first, then the next and the
  /// one before; one drawn smaller than wanted is drawn anew.
  int? _next() {
    for (final index in [_page, _page + 1, _page - 1]) {
      if (index < 0 || index >= widget.pages.length) continue;
      if (_failed.contains(index)) continue;
      final drawn = _drawn[index];
      if (drawn == null) return index;
      final wanted = _wanted(index);
      if (wanted >= (_ceiling[index] ?? wanted + 1)) continue;
      if (drawn.width * 1.25 < wanted) return index;
    }
    return null;
  }

  /// Whether the page [index] drawn [width] wide is still wanted once it
  /// comes: not when the pages turned away from it, or it was drawn for
  /// a zoom left since.
  bool _stillWanted(int index, int width) =>
      (index - _page).abs() <= 1 && width <= _wanted(index) * 1.25;

  /// Draws the pages wanted, one at a time, so at most one page is
  /// being drawn in memory.
  Future<void> _draw() async {
    if (_drawing) return;
    _drawing = true;
    try {
      while (mounted) {
        final index = _next();
        if (index == null) return;
        final width = _wanted(index);
        final png = await _files.pdfRender(widget.path, index, width);
        if (!mounted) return;
        if (png == null) {
          setState(() {
            if (_drawn.containsKey(index)) {
              _ceiling[index] = width;
            } else {
              _failed.add(index);
            }
          });
        } else if (_stillWanted(index, width)) {
          setState(() {
            _forget(index);
            _drawn[index] = (width: width, png: png);
          });
        }
      }
    } finally {
      _drawing = false;
    }
  }

  /// Lets go of the page [index] drawn, and of its decoded picture.
  void _forget(int index) {
    final drawn = _drawn.remove(index);
    if (drawn != null) unawaited(MemoryImage(drawn.png).evict());
  }

  void _turned(int page) {
    setState(() {
      _page = page;
      _scale = 1;
      _zoom.value = Matrix4.identity();
      // Pages out of reach go, and a neighbour drawn zoomed goes back to
      // the size it shows at.
      for (final index in _drawn.keys.toList()) {
        if ((index - page).abs() > 1 ||
            (index != page && _drawn[index]!.width > _wanted(index) * 1.25)) {
          _forget(index);
        }
      }
    });
    unawaited(_draw());
  }

  void _zoomEnded(ScaleEndDetails _) {
    setState(() {
      _scale = _zoom.value.getMaxScaleOnAxis();
      // Barely zoomed is back at its size, so the page turns again.
      if (_scale < 1.1) {
        _scale = 1;
        _zoom.value = Matrix4.identity();
      }
    });
    unawaited(_draw());
  }

  Future<void> _openElsewhere() async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    _handed = true;
    if (await _files.open(widget.path, widget.file.mediaType)) return;
    announceFailureWith(
      announcements,
      navigator,
      t.fileNotOpened,
      title: t.fileNotOpenedTitle,
      cause: t.fileNoAppCause,
      fix: t.fileNoAppFix,
      details: 'file ${widget.file.hash}: no app took ${widget.file.mediaType}',
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final count = widget.pages.length;
    return Scaffold(
      appBar: pageBar(
        context,
        title: aloneFileTitle(context, widget.file),
        actions: [
          if (_hasText)
            IconButton(
              key: const ValueKey('pdf-text'),
              tooltip: t.attachmentTextLabel,
              isSelected: _showText,
              icon: const Icon(Icons.subject),
              onPressed: () => setState(() => _showText = !_showText),
            ),
          PopupMenuButton<bool>(
            key: const ValueKey('pdf-menu'),
            tooltip: t.more,
            icon: const Icon(Icons.more_vert),
            onSelected: (_) => unawaited(_openElsewhere()),
            itemBuilder: (_) => [
              PopupMenuItem<bool>(
                key: const ValueKey('pdf-open'),
                value: true,
                child: Row(
                  children: [
                    const Icon(Icons.open_in_new),
                    const SizedBox(width: 12),
                    Flexible(child: Text(t.openInOtherApp)),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
      body: SafeArea(
        child: _showText
            ? _textView(context)
            : Column(
                children: [
                  if (count > 1)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: Text(
                        t.pdfPageOf(_page + 1, count),
                        key: const ValueKey('pdf-page-of'),
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                    ),
                  Expanded(
                    child: PageView.builder(
                      controller: _pager,
                      // A zoomed page pans; it turns again at its size.
                      physics: _zoomed
                          ? const NeverScrollableScrollPhysics()
                          : null,
                      itemCount: count,
                      onPageChanged: _turned,
                      itemBuilder: _pageAt,
                    ),
                  ),
                ],
              ),
      ),
    );
  }

  Widget _pageAt(BuildContext context, int index) {
    final size = widget.pages[index];
    final drawn = _drawn[index];
    final shown = index == _page;
    return InteractiveViewer(
      key: ValueKey('pdf-zoom-$index'),
      transformationController: shown ? _zoom : null,
      panEnabled: shown && _zoomed,
      maxScale: 6,
      onInteractionEnd: shown ? _zoomEnded : null,
      child: Center(
        child: Semantics(
          image: true,
          label: context.t.pdfPageOf(index + 1, widget.pages.length),
          child: AspectRatio(
            aspectRatio: size.width / size.height,
            child: ColoredBox(
              color: Colors.white,
              child: drawn != null
                  ? Image.memory(
                      drawn.png,
                      key: ValueKey('pdf-page-$index'),
                      fit: BoxFit.contain,
                      gaplessPlayback: true,
                    )
                  : _failed.contains(index)
                  ? const Center(
                      child: Icon(
                        Icons.broken_image_outlined,
                        color: Colors.black54,
                      ),
                    )
                  : const SizedBox.expand(),
            ),
          ),
        ),
      ),
    );
  }

  Widget _textView(BuildContext context) {
    final text = widget.text;
    return SingleChildScrollView(
      key: const ValueKey('pdf-text-view'),
      padding: const EdgeInsets.all(16),
      child: text == null || text.isEmpty
          ? AttachmentText(file: widget.file, text: text)
          : Text(
              text,
              key: ValueKey('pdf-text:${widget.file.hash}'),
              style: Theme.of(context).textTheme.bodyLarge,
            ),
    );
  }
}
