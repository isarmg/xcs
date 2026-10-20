# 历史制作记录：V1

以下为已提交的第一版制作记录；另有 [V2 短样片记录](README-v2.md)。当前交付按本文上方 V3 命令重建。

220 秒（3 分 40 秒）、12 个场景，从文件混乱的痛点出发，说明完整版本、冻结引用、分支首次写入、目录组合、显式合并、逐盘备份与恢复边界。使用概念动画与真实 Web 画面说明项目的核心流程。

## 素材与源码

- `story-timed.json`：最终 220 秒时间轴和真实口播逐句字幕时间，默认重建使用此文件。
- `story.json`：同一终版时间轴的文案源（不附逐句口播时间）。
- `STORYBOARD.md`：逐场视觉说明。
- `NARRATION.md`：完整中文旁白稿。
- `render.py`：Pillow 场景图、NumPy 原创环境配乐、ffmpeg 视频编码。
- `../../docs/images/web-file-manager.png`：项目本地实例的真实 Web 截图；演示文件为合成数据。视频仅裁切截图上部，以提高可读性，未伪造界面。
- `voice.py` / `VOICE.md`：本地中文合成旁白生成与模型许可说明。最终成片含合成旁白。
- `captions.zh-CN.srt`：33 句可编辑字幕，按真实口播时码导出；MP4 同时保留烧录字幕。
- `export_subtitles.py`：从最终时间轴重建 SRT。

图形与配乐均通过代码原创生成，无外部音乐或图标素材。中文字体使用系统 Noto Sans CJK（SIL Open Font License），字体未复制进仓库。截图和项目界面的许可遵循项目原有许可。

## 环境

此目录是独立的视频制作工具，使用 Python，不参与 Rust 产品构建。

Python 3.10+，`ffmpeg` / `ffprobe`（系统软件），系统 Noto Sans CJK Regular/Bold TTC 字体（默认 `/usr/share/fonts/opentype/noto`）。

```sh
python -m pip install -r media/intro-video/requirements.txt
# 先输出各场中点帧与总览图（不生成视频）
python media/intro-video/render.py --out ../zxs-video-output --width 1280 --preview
# 成片：1920×1080 / 30 fps / 220 秒（3 分 40 秒），带原创轻量背景配乐
python media/intro-video/render.py --out ../zxs-video-output
```

输出目录必须在 Git 仓库之外。`--width 960` 可用于快速预览；`--duration 10` 可用于编码测试。终渲染会拒绝缺少真实截图的情况。视频采用 RGB 帧经标准输入直接交给 ffmpeg 编码，不产生海量帧文件。仅保留 12 张预览 JPEG 与 contact sheet。

旁白标准化至 -18 LUFS，配乐降至 18% 后混音并限制峰值；先检查音画时长一致。配乐混入已对齐的 220 秒（3 分 40 秒）旁白（可选；旁白生成成功后使用）：

```sh
python media/intro-video/mux_voice.py \
  --video ../zxs-video-output/zxs-explainer-zh.mp4 \
  --voice ../zxs-video-output/voice/narration.wav \
  --out ../zxs-video-output/zxs-explainer-zh-narrated.mp4
```

音视频、模型、虚拟环境及临时产物不提交 GitHub。`.gitignore` 排除常见媒体与渲染缓存；仍应在提交前检查暂存区。

## 检查

```sh
cd media/intro-video
python -m unittest -v test_render.py
```

测试检查时间轴连续性、场景渲染、字幕/步骤容量及动画变化。交付前另用 ffprobe 检查分辨率/帧率/时长，并从编码后视频解码首、中、尾画面进行目视检查。

合成旁白采用自然语速时，可根据实际音频扩展场景，再通过 `--story /path/to/story-timed.json` 重渲染。`duration` 与各场 `start` / `end` 使用更新后的秒数；每场可增加 `caption_timings: [{"start": 0.4, "end": 4.2, "text": "逐句字幕"}]`，时间为全片绝对秒数。这样字幕按实际口播切换，而非平均分配。静音间隔保留上一句。

从 `voice.py` 的实际逐句报告附加字幕时间：

```sh
python media/intro-video/align_captions.py --story /path/to/approved-story.json \
  --report ../zxs-video-output/voice/voice-report.json \
  --out ../zxs-video-output/story-timed.json
python media/intro-video/render.py --story ../zxs-video-output/story-timed.json \
  --out ../zxs-video-output/final
```

此工具不自行更改场景时长；任何语音超出场景的情况都会明确报错，先调整已批准的时间轴再渲染。
