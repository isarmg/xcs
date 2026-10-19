#!/usr/bin/env python3
"""Make two short v2 voice auditions without changing v1 audio or its script."""
import argparse
import hashlib
import json
import os
from pathlib import Path

RATE = 24000
MODEL = "hexgrad/Kokoro-82M-v1.1-zh"
VOICES = ("zm_010", "zf_017")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--script", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--speed", type=float, default=1.14)
    args = parser.parse_args()
    if not 1.0 <= args.speed <= 1.2:
        parser.error("Use a moderate synthesis speed between 1.0 and 1.2")
    args.output.mkdir(parents=True, exist_ok=True)
    os.environ["HF_HOME"] = str(args.cache.resolve())
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    import numpy as np
    import soundfile as sf
    import torch
    from kokoro import KModel, KPipeline
    torch.set_num_threads(4)
    torch.manual_seed(42)
    source = args.script.read_bytes()
    script = json.loads(source)
    if "scenes" in script:
        texts = [s["narration"] for s in script["scenes"]]
    else:
        sentences = script["sentences"]
        texts = [s if isinstance(s, str) else s["text"] for s in sentences]
    english = {"XCS": "ˈɛks sˈi ˈɛs", "Web": "wˈɛb", "B": "bˈi"}
    def pronounce(text):
        if text not in english:
            raise ValueError(f"Review pronunciation for {text!r}")
        return english[text]
    model = KModel(repo_id=MODEL).to("cpu").eval()
    pipeline = KPipeline(lang_code="z", repo_id=MODEL, model=model, en_callable=pronounce)
    reports = []
    for voice in VOICES:
        pieces = []
        timings = []
        cursor = 0
        for index, text in enumerate(texts):
            if index:
                gap = np.zeros(round(.06 * RATE), dtype=np.float32)
                pieces.append(gap)
                cursor += len(gap)
            chunks = [r.audio.numpy() for r in pipeline(text, voice=voice, speed=args.speed, split_pattern=None)
                      if r.audio is not None]
            if not chunks:
                raise RuntimeError(f"No audio for sentence {index + 1}")
            samples = np.concatenate(chunks)
            timings.append({"index": index + 1, "text": text, "start": cursor / RATE,
                            "end": (cursor + len(samples)) / RATE})
            pieces.append(samples)
            cursor += len(samples)
        audio = np.concatenate(pieces)
        peak = float(np.max(np.abs(audio)))
        rms = float(np.sqrt(np.mean(audio ** 2)))
        # Same modest amplitude target for a fair comparison; no time stretching.
        gain = min(10 ** (-22 / 20) / max(rms, 1e-9), .9 / max(peak, 1e-9))
        audio *= gain
        path = args.output / f"audition-{voice}.wav"
        sf.write(path, audio, RATE, subtype="PCM_16")
        report = {"voice": voice, "speed": args.speed, "duration": len(audio) / RATE,
                  "sample_rate": RATE, "file": path.name, "timings": timings,
                  "peak": float(np.max(np.abs(audio))), "gain": gain,
                  "auditory_review": "Not performed; user audition required"}
        reports.append(report)
        print(json.dumps(report, ensure_ascii=False), flush=True)
    report = {"model": MODEL, "script_sha256": hashlib.sha256(source).hexdigest(), "candidates": reports}
    (args.output / "audition-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
