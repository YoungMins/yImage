<div align="center">

# yImage

**Rust 製の高速 Windows 画像ビューアー & エディター。**

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md)

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/youngminkim)

</div>

---

yImage は「既定のアプリに設定して忘れる」タイプの Windows 画像ビューアーを
Rust + `egui` + `wgpu` で書き直したものです。数ミリ秒で起動し、負荷の高い
処理はすべて SIMD で最適化しています。

## 機能

- **爆速ビューアー** — 300 ms 以下のコールドスタート、GPU 加速のズーム/パン、
  同一フォルダー内の画像をキーボードで順送り。
- **既定の画像ビューアーに設定** — `ファイル → 既定の画像ビューアーに設定`
  メニューから、管理者権限なしのユーザー単位でファイルの関連付けを登録。
- **容量の最適化** — `oxipng` によるロスレス PNG 最適化、`mozjpeg` による高品質
  JPEG、WebP の品質スライダー。
- **サイズ変更** — `fast_image_resize` による SIMD Lanczos / Bilinear / Nearest。
- **フォーマット変換** — PNG · JPEG · WebP · BMP · TIFF · GIF · AVIF、並列
  バッチ変換に対応。
- **編集機能**
  - 描画 — サイズ / 硬さ / カラーを調整できるアンチエイリアスブラシ。
  - モザイク — 矩形領域を任意のブロックサイズでピクセル化。
  - 背景削除 — ローカルの U²-Net ONNX モデル (クラウド送信/テレメトリーなし)。
  - オブジェクト削除 — マスクを塗って LaMa インペインティングで復元。
- **画面キャプチャ** — Windows Graphics Capture API でコンポジターからゼロ
  コピー。キャプチャ結果はそのまま新しいドキュメントとして開きます。
- **GIF 作成** — 複数画像からフレーム間隔と NeuQuant 量子化で GIF を生成。
- **3 言語 UI** — English / 한국어 / 日本語、ライブ切り替え対応。

## インストール

[Releases](https://github.com/youngmins/yimage/releases) から
`yImage-Setup-x.y.z.exe` をダウンロードして実行してください。

インストーラーは以下を行います:

- `%ProgramFiles%\yImage` へ yImage をインストール。
- 一般的な画像フォーマットの「プログラムから開く」候補に yImage を追加し、
  Windows 11 の *設定 → アプリ → 既定のアプリ* に表示されるよう構成。
- 背景削除 / オブジェクト削除に必要な ONNX モデルを同梱。

## ソースからビルド

```powershell
git clone https://github.com/youngmins/yimage.git
cd yimage
cargo build --release
.\target\release\yimage.exe
```

要件: Rust stable (≥ 1.78)、画面キャプチャを使用する場合は Windows 10 1903 以降。

インストーラーのビルド:

```powershell
# ONNX モデルのダウンロード — assets/models/README.md を参照
iscc installer\yImage.iss
```

## テクノロジースタック

| 分野 | クレート |
|---|---|
| GUI / キャンバス | `eframe` + `egui` + `wgpu` |
| 画像 IO | `image` |
| リサイズ | `fast_image_resize` |
| 描画 | `tiny-skia` + `imageproc` |
| PNG 最適化 | `oxipng` |
| JPEG 最適化 | `mozjpeg` |
| WebP | `webp` |
| GIF | `gif` + `color_quant` |
| ONNX 推論 | `ort` (ONNX Runtime) |
| 画面キャプチャ | `windows-capture` |
| 国際化 | `fluent` |

## サポート

yImage がお役に立てましたら、Ko-fi で開発を応援していただけると嬉しいです ☕

<a href="https://ko-fi.com/youngminkim"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Ko-fi でサポート" /></a>

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
