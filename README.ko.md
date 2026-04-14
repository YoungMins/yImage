<div align="center">

# yImage

**Rust로 만든 빠른 Windows 이미지 뷰어 & 편집기.**

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md)

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/youngminkim)

</div>

---

yImage는 "기본 프로그램으로 한 번 지정해 두고 잊어버리는" 고전적인 이미지
뷰어의 컨셉을 Rust + `egui` + `wgpu` 기반으로 재작성한 프로그램입니다. 수 ms
안에 실행되며, 성능이 중요한 부분은 전부 SIMD로 처리합니다.

## 기능

- **압도적으로 빠른 뷰어** — 300 ms 이하의 콜드 스타트, GPU 가속 확대/이동,
  폴더 내 다음/이전 이미지 키보드 이동.
- **기본 이미지 뷰어로 설정** — `파일 → 기본 이미지 뷰어로 설정` 메뉴 한 번이면
  사용자 단위로 파일 연결 등록. 관리자 권한 불필요.
- **용량 최적화** — `oxipng`로 무손실 PNG 최적화, `mozjpeg`로 고품질 JPEG,
  WebP 품질 슬라이더.
- **크기 조정** — `fast_image_resize` 기반 SIMD Lanczos/Bilinear/Nearest.
- **포맷 변환** — PNG · JPEG · WebP · BMP · TIFF · GIF · AVIF, 병렬 일괄 변환
  지원.
- **편집 기능**
  - 드로잉 — 크기 / 경도 / 색상을 조절할 수 있는 안티에일리어싱 브러시.
  - 모자이크 — 사각 영역을 지정한 블록 크기로 픽셀화.
  - 배경 제거 — 로컬 U²-Net ONNX 모델 (클라우드 전송/텔레메트리 없음).
  - 객체 제거 — 마스크를 칠한 뒤 LaMa 인페인팅으로 복원.
- **화면 캡처** — Windows Graphics Capture API로 DWM에서 제로카피. 캡처 결과가
  곧바로 새 문서로 열립니다.
- **GIF 만들기** — 여러 이미지에서 프레임 간격과 NeuQuant 컬러 양자화로 GIF
  생성.
- **3개 국어 UI** — English / 한국어 / 日本語, 실시간 전환 가능.

## 설치

[Releases](https://github.com/youngmins/yimage/releases) 페이지에서
`yImage-Setup-x.y.z.exe`를 받아 실행하세요.

설치 프로그램이 하는 일:

- `%ProgramFiles%\yImage`에 yImage를 설치합니다.
- 일반 이미지 포맷의 "열 수 있는 앱" 목록에 yImage를 등록하고, Windows 11의
  *설정 → 앱 → 기본 앱* 에 노출되도록 구성합니다.
- 배경 제거 / 객체 제거에 필요한 ONNX 모델을 함께 설치합니다.

## 소스에서 빌드

```powershell
git clone https://github.com/youngmins/yimage.git
cd yimage
cargo build --release
.\target\release\yimage.exe
```

요구 사항: Rust stable (≥ 1.78), 화면 캡처를 쓰려면 Windows 10 1903 이상.

설치 프로그램 빌드:

```powershell
# ONNX 모델 다운로드 — assets/models/README.md 참고
iscc installer\yImage.iss
```

## 기술 스택

| 영역 | 크레이트 |
|---|---|
| GUI / 캔버스 | `eframe` + `egui` + `wgpu` |
| 이미지 입출력 | `image` |
| 리사이즈 | `fast_image_resize` |
| 드로잉 | `tiny-skia` + `imageproc` |
| PNG 최적화 | `oxipng` |
| JPEG 최적화 | `mozjpeg` |
| WebP | `webp` |
| GIF | `gif` + `color_quant` |
| ONNX 추론 | `ort` (ONNX Runtime) |
| 화면 캡처 | `windows-capture` |
| 국제화 | `fluent` |

## 후원

yImage가 시간을 아껴 드렸다면 Ko-fi로 개발을 응원해 주세요 😊

<a href="https://ko-fi.com/youngminkim"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Ko-fi로 후원하기" /></a>

## 라이선스

MIT — [LICENSE](LICENSE) 파일을 참고하세요.
