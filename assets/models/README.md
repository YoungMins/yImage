# yImage ONNX Models

yImage ships with two local ONNX models for its AI features. They are **not**
committed to git because of their size — the GitHub Actions release workflow
downloads them into this directory before running the installer build.

| File | Purpose | Approx size | Source |
|---|---|---|---|
| `u2netp.onnx` | Background removal (U²-Net tiny) | ~4.6 MB | https://github.com/danielgatis/rembg/releases |
| `lama-fp16.onnx` | Object removal / inpainting (LaMa) | ~100 MB | https://github.com/enesmsahin/simple-lama-inpainting/releases |

## Download manually

```powershell
mkdir -Force assets/models
Invoke-WebRequest -Uri <u2netp-url> -OutFile assets/models/u2netp.onnx
Invoke-WebRequest -Uri <lama-url>   -OutFile assets/models/lama-fp16.onnx
```

If the models are missing at runtime, the matching UI action will surface a
clear error instead of crashing. Everything else in yImage keeps working
without the models.
