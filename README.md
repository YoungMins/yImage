<div align="center">

# yImage

**Fast Windows image viewer and editor written in Rust.**

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md)

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/youngminkim)

</div>

---

yImage is a ground-up rewrite of a classic "set as default and forget it"
Windows image viewer, built on Rust + `egui` + `wgpu` so it starts in
milliseconds and uses SIMD everywhere that matters.

## Features

- **Blazing-fast viewer** — sub-300 ms cold start, GPU-accelerated zoom and pan,
  keyboard navigation between images in the same folder.
- **Set as default image viewer** — one-click file association via the
  `File → Set as Default Image Viewer` menu (per-user, no admin prompt).
- **Size optimisation** — lossless PNG via `oxipng`, high-quality JPEG via
  `mozjpeg`, tunable WebP quality.
- **Resize** — SIMD-accelerated Lanczos/Bilinear/Nearest via `fast_image_resize`.
- **Format conversion** — PNG · JPEG · WebP · BMP · TIFF · GIF · AVIF, with
  parallel batch support.
- **Editing**
  - Draw — anti-aliased brush with size / hardness / colour.
  - Mosaic — pixelate a rectangular region with adjustable block size.
  - Background removal — local U²-Net ONNX model (no cloud, no telemetry).
  - Object removal — LaMa inpainting over a brushed mask.
- **Screen capture** — Windows Graphics Capture API, zero-copy from the
  compositor. Capture opens directly as a new document.
- **GIF creation** — build animated GIFs from a sequence of images with
  per-frame delay and NeuQuant quantisation.
- **Trilingual UI** — English / 한국어 / 日本語, with live language switching.

## Install

Grab `yImage-Setup-x.y.z.exe` from the
[Releases](https://github.com/youngmins/yimage/releases) page and run it.

The installer:

- Installs yImage to `%ProgramFiles%\yImage`.
- Registers yImage as an available opener for common image formats and adds it
  to Windows 11 *Settings → Apps → Default apps*.
- Bundles the ONNX models needed for background / object removal.

## Build from source

```powershell
git clone https://github.com/youngmins/yimage.git
cd yimage
cargo build --release
.\target\release\yimage.exe
```

Requirements: Rust stable (≥ 1.78), Windows 10 1903+ for screen capture.

For the installer:

```powershell
# Download ONNX models — see assets/models/README.md
iscc installer\yImage.iss
```

## Tech stack

| Area | Crate |
|---|---|
| GUI / canvas | `eframe` + `egui` + `wgpu` |
| Image IO | `image` |
| Resize | `fast_image_resize` |
| Drawing | `tiny-skia` + `imageproc` |
| PNG optimise | `oxipng` |
| JPEG optimise | `mozjpeg` |
| WebP | `webp` |
| GIF | `gif` + `color_quant` |
| ONNX inference | `ort` (ONNX Runtime) |
| Screen capture | `windows-capture` |
| i18n | `fluent` |

## Support

If yImage saves you time, please consider supporting development on Ko-fi:

<a href="https://ko-fi.com/youngminkim"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Support me on Ko-fi" /></a>

## License

MIT — see [LICENSE](LICENSE).
