# V2 动作样片

这是待用户验证叙事节奏与声音的短样片，不是完整成片。保留 V1 源码，V2 独立实现；不会把原 28 秒动作模板的空白重新加回配音。

- 单文件：宣传方案.txt
- 明确改动：公众号 → 误改短视频 → 查看旧版公众号 → 创建方案 B → 上传公众号＋社群 → 双向切换验证
- 画面标示“操作逻辑演示 · 非实机录屏”；内容编辑为示意，真实 Web 入口通过上传写入当前分支
- 9 段动作按裁剪静音后的真实口播段落映射；开头 0.6 秒，结尾 0.8 秒
- 首轮有声样片：zm_010，约 21.63 秒，无背景音乐

## 重建

语音生成与仅裁句首/句尾静音的方法见 `VOICE_V2.md`。`voice-v2-timings.json` 保留本次选择声音的实际分段时间；未包含音频或权重。

```sh
python media/intro-video/render_v2.py \
  --out ../xcs-video-output/v2-final-sample \
  --voice-report media/intro-video/voice-v2-timings.json \
  --voice-id zm_010 \
  --audio ../xcs-video-output/voice-v2/audition-zm_010-trimmed.wav
```

`story-v2.json` 的 28 秒时标是动作编排模板，最终依照每句语音的实际时长线性映射对应动作段，不改变句内语音。最终默认 1920×1080、30fps、H.264/AAC，含烧录字幕。输出视频与关键帧必须放在仓库外。
