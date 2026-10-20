# 中文旁白制作

`voice.py` 在本机 CPU 合成中文旁白，不调用在线语音接口，也不上传脚本。
默认读取 `story-timed.json` 的 `caption`；输出按同一文件的 `start` / `end` 对齐。

## 来源与许可核验（2026-10-19）

- 模型：[hexgrad/Kokoro-82M-v1.1-zh 官方模型卡](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh)。发布者标注 Apache-2.0；模型卡说明中文专业语料由龙猫数据授权提供。
- 推理库：[hexgrad/kokoro](https://github.com/hexgrad/kokoro)，Apache-2.0；安装官方 PyPI `kokoro` 与 `misaki[zh]`。
- 默认声音 `zf_001`，24 kHz。它是公开模型的预设声音，不是为本项目克隆某个人的声音。
- 权重与软件不复制进此仓库。模型 SHA256：`b1d8410fa44dfb5c15471fd6c4225ea6b4e9ac7fa03c98e8bea47a9928476e2b`（已核对）。

## 重建

在仓库外建立虚拟环境与输出目录。例如：

```sh
python -m venv ../zxs-video-output/voice-venv
../zxs-video-output/voice-venv/bin/pip install torch --index-url https://download.pytorch.org/whl/cpu
../zxs-video-output/voice-venv/bin/pip install 'kokoro==0.9.4' 'misaki[zh]==0.9.4' soundfile 'transformers==4.57.6' 'huggingface-hub<1'
../zxs-video-output/voice-venv/bin/python media/intro-video/voice.py --story media/intro-video/story-timed.json --output ../zxs-video-output/voice
```

首跑从官方 Hugging Face 模型仓库下载权重和声音；之后可复用本地缓存。
`--scene s01` 只生成一场试听；`--speed` 为生成速度，不是事后音频变速。

每场在画面开始后 0.35 秒起声，结束前预留 0.35 秒。
`voice-report.json` 记录时长、峰值、RMS、文案哈希及超时场景。
如果任何场景超时，脚本会退出并拒绝写成完整混音，不会截断句子或硬塞过快语速。
应先缩短该场文案或调整画面时长，再重新生成，并同步字幕。

生成文件：逐场 `s01.wav` 等；全部通过时长检查后生成整段 `narration.wav`。
WAV、模型、虚拟环境、最终视频均不提交 Git。

## 成片检查

采用自然语速 `1.0`，为完整旁白将时间轴扩展到 220 秒。原始口播总长约 200.345 秒，余下为场景间停顿。校验包括：逐场长度、有效音频信号、峰值与无截句；未进行人工听音审校。可先试听逐场 WAV，再按需要调整发音或停顿。

`voice-report.json` 的 `caption_timings_relative` 含各合成句相对场景起点的 start/end/text；与场景 start 相加即可同步字幕。
