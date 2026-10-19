# XCS 中文解释动画

当前版：54秒、1920×1080、30 fps、无配音。同一份活动方案从8→10→12留下历史；从旧8建立精简方案，首次写入6，主12与精简6并存，再演示完整文件分开存放。

## 重建

需要 Python 3.10+、ffmpeg/ffprobe、系统 Noto Sans CJK Regular/Bold 字体。以下命令从仓库根目录运行：

```sh
python -m pip install -r media/intro-video/requirements.txt
python media/intro-video/render_v3_animation.py --render --out ../xcs-video-output/v3/final
```

默认读取随源码保存的真实截图。省略 `--render` 生成关键帧；`--duration 27 --width 1280` 可输出前半段预览。视频与检查结果保存在仓库外。

## 测试与检查

```sh
(cd media/intro-video && python -m unittest test_render_v3_animation.py)
python media/intro-video/qa_v3.py \
  --video ../xcs-video-output/v3/final/xcs-v3-content-silent.mp4 \
  --out ../xcs-video-output/v3/final/qa
```

检查包括实际解码、黑帧、帧数、分镜边界前后帧及ffprobe信息。静态帧和指标用于技术检查，节奏仍需连续观看评价。

## 素材与制作源

原创机制动画配合经真实Chrome/XCS流程验证的正文截图；两盘为存放位置示意。截图使用合成文件数据，原始PNG及采集来源、验证记录保存在 `assets/capture/`。字体遵循 Noto Sans CJK 的 SIL Open Font License。

- `story-v3.json`、`redesign-v3.md`：54秒分镜与对象衔接
- `render_v3_animation.py`：连续文档、引用、首次写入与存放动画
- `render.py`：共用画布、中文排版及颜色工具
- `qa_v3.py`、`test_render_v3_animation.py`：解码检查与单元测试

历史制作记录：[V1](README-v1.md) · [V2](README-v2.md)
