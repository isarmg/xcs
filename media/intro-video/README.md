# ZXS 分支动画

当前版：10.5秒，1920×1080，30 fps，无配音。从旧8另开方案，写成6，主方案12保持不变。画面为原创机制动画。

## 重建

需要 Python 3.10+、ffmpeg/ffprobe、系统 Noto Sans CJK 字体。从仓库根目录运行：

```sh
python -m pip install -r media/intro-video/requirements.txt
python media/intro-video/render_v4_branch.py --render --out ../zxs-video-output/v4-branch
```

省略 `--render` 生成六张关键帧。源码复用 `render.py` 的排版与 `render_v3_animation.py` 的文档绘制。

## 检查

```sh
(cd media/intro-video && python -m unittest test_render_v4_branch.py)
python media/intro-video/qa_v3.py --story media/intro-video/story-v4.json \
  --video ../zxs-video-output/v4-branch/zxs-branch.mp4 \
  --out ../zxs-video-output/v4-branch/qa
```

检查实际解码、黑帧、帧数和动作交界前后帧。技术检查不替代连续观看。

历史记录：[V1](README-v1.md) · [V2](README-v2.md) · [V3](README-v3.md)
