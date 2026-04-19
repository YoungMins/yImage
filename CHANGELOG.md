# Changelog

All notable changes to yImage are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-04-19

A performance-focused release. Large folders, big photos, and arrow-key
navigation are all substantially faster, and several UX rough edges from
0.1.0 have been smoothed over.

### Added
- **Adjacent-image prefetch** — after an image loads, the previous and
  next siblings in the folder are decoded on a background thread and
  stashed in a tiny LRU, so arrow-key / thumbnail navigation skips the
  full decode.
- **Persistent on-disk thumbnail cache** under the platform cache
  directory. Keyed by path + mtime so stale entries auto-invalidate;
  survives restarts, which means returning to a photo folder is instant.
- **Background folder preheat** — when a folder is scanned and the
  thumbnail panel is visible, up to 64 thumbnails are pre-decoded to
  the disk cache in the background.
- **Shared decode** — the viewer's decoded image is reused to produce
  the filmstrip thumbnail, eliminating a redundant decode.
- **Auto-downsample for large images** — images larger than 8192 px in
  any dimension are downscaled before GPU texture upload, avoiding the
  silent failures many GPU backends exhibit above that size. The full
  resolution buffer is kept for editing and export.
- **Multi-file drag & drop** — drop multiple files onto the window and
  each opens as a separate tab.

### Changed
- **Zero-copy RGBA → Color32 upload** for fully opaque images via
  `bytemuck::cast_slice`. On a 24 MP photo this replaces roughly 24 M
  per-pixel calls with a single `memcpy`.
- **Bounded thumbnail cache** — the in-memory thumbnail cache is now an
  LRU capped at 256 entries, so a folder of tens of thousands of photos
  can't pin unbounded GPU memory.
- **Parallel GIF frame processing** — resize and quantization now run
  across all CPU cores via rayon; the encoder stays sequential because
  it isn't thread-safe.
- Ribbon toolbar gets thin hairlines above and below instead of a
  distinct fill, keeping the panel unified with the window chrome.
- Welcome screen recent files are horizontal cards (thumbnail on top,
  filename below) rather than a vertical list.

### Fixed
- Object-removal progress indicator no longer stays visible after the
  operation completes.
- Object-removal output range is auto-detected; both `[0, 1]` and
  `[0, 255]` LaMa ONNX exports now produce correct (non-white) fills.
- Context sub-toolbar no longer has rounded corners at the ends.
- Object mask is cleared when switching images, preventing a mask from
  one document from painting over another.

## [0.1.0] - Initial release

First public build. Fast Windows image viewer + editor with:

- GPU-accelerated viewer, keyboard folder navigation.
- Set as default image viewer (per-user, no admin prompt).
- Size optimisation (PNG via oxipng, JPEG via mozjpeg, tunable WebP).
- SIMD-accelerated resize (fast_image_resize).
- Format conversion across PNG / JPEG / WebP / BMP / TIFF / GIF / AVIF.
- Editing tools: brush, mosaic, text, shapes.
- Background removal (U²-Net) and object removal (LaMa) via bundled ONNX models.
- Screen capture using Windows Graphics Capture API.
- GIF creation from image sequences.
- Trilingual UI — English / 한국어 / 日本語.

[Unreleased]: https://github.com/youngmins/yimage/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/youngmins/yimage/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/youngmins/yimage/releases/tag/v0.1.0
