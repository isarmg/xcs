# v2 短样片声音候选

先用同一段口语稿做两份试听，用户选定后再制作全片。旧版音轨不覆盖。
候选不是自然度已验收的声音；当前制作环境不能听音，需实际试听决定。

- 模型：[hexgrad/Kokoro-82M-v1.1-zh](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh)，官方标注 Apache-2.0。
- 候选 A：`zm_010`，在[官方中文示例](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh/blob/main/samples/make_zh.py)明确列出。
- 候选 B：`zf_017`，在[官方 voices 清单](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh/tree/main/voices)明确列出。
- 两者使用同一脚本、生成速度 `1.14`，句间只额外添加 0.06 秒，不做音频拉伸。
- 音量采用相同 RMS 目标，并限制峰值；不做夸大表情或仿冒特定人物。

声音文件官方 SHA256：

| 预设 | SHA256 |
| --- | --- |
| zm_010 | d2eeba86192eee269f600ca6821038034abd017532a1fe68ff7b0e86c2983b2a |
| zf_017 | e507bb858a75f0518f08918827740530a656a008e3c057435a17b3a95f267624 |

依赖安装参见 `VOICE.md`。输入使用 `story-v2.json` 的 `scenes[].narration`，也支持 `sentences` 数组（字符串或 `{ "text": "一句话。" }`）。

```sh
../xcs-video-output/voice-venv/bin/python media/intro-video/voice_v2.py \
  --script media/intro-video/story-v2.json \
  --output ../xcs-video-output/voice-v2 \
  --cache ../xcs-video-output/voice/model-cache
```

输出两份 WAV 和 `audition-report.json`，含实际时长、逐句字幕切点、语速、信号检查与脚本哈希。仅源码提交 Git；音频、权重、缓存留在仓库外。

## 本次短样片实测

- A / zm_010：27.005 秒，峰值 0.741。
- B / zf_017：28.680 秒，峰值 0.869。
- 两者均为 24 kHz、单声道、PCM16；声音张量哈希与官方清单一致。
- 仅完成文件、时长和信号检查。没有听音验收，不据此宣称自然度优于旧版。

### 去除每句合成自带的首尾空白

模型给每句添加了约 0.25 秒前空白和 0.65 秒后空白。`trim_voice_v2.py` 只剪两端低声能区域：以 10ms 窗口 RMS ≤ -50dBFS 判定，首尾各保留 60ms，额外句间间隔仍为 60ms。保留全部句内停顿，不改变语速或音高；原 WAV 保留。

```sh
../xcs-video-output/voice-venv/bin/python media/intro-video/trim_voice_v2.py \
  --directory ../xcs-video-output/voice-v2
```

| 候选 | 裁剪前 | 裁剪后 | 低声能占比，前→后 |
| --- | --- | --- | --- |
| A / zm_010 | 27.005s | 20.210s | 35.9% → 14.5% |
| B / zf_017 | 28.680s | 21.520s | 37.0% → 16.0% |

这些是声能阈值统计，不是语义级语音识别或听感评估。使用 `audition-*-trimmed.wav` 与 `audition-trimmed-report.json` 的新时间，不能套用裁剪前时间轴。
