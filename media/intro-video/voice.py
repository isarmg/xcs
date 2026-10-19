#!/usr/bin/env python3
"""Local Mandarin narration. Weights/cache and rendered audio stay outside git.

See VOICE.md for dependencies, publisher, licensing and reproducibility notes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path

RATE = 24000
REPO_ID = "hexgrad/Kokoro-82M-v1.1-zh"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--story", type=Path, default=Path(__file__).with_name("story-timed.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--voice", default="zf_001")
    parser.add_argument("--speed", type=float, default=1.0)
    parser.add_argument("--scene", help="Generate only one scene as an audition")
    args = parser.parse_args()
    if args.speed <= 0:
        parser.error("--speed must be positive")
    args.output.mkdir(parents=True, exist_ok=True)
    # A failed rebuild must not leave an older mix looking like a fresh result.
    if not args.scene:
        (args.output / "narration.wav").unlink(missing_ok=True)
    os.environ.setdefault("HF_HOME", str(args.output / "model-cache"))
    os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")
    import numpy as np
    import soundfile as sf
    import torch
    from kokoro import KModel, KPipeline
    torch.set_num_threads(4)
    torch.manual_seed(42)
    # Only these English tokens occur in the frozen Mandarin script. Explicit
    # phonemes avoid a second English model and keep acronyms intelligible.
    english = {"XCS": "ˈɛks sˈi ˈɛs", "Web": "wˈɛb", "FULL": "fˈʊl"}
    def en_callable(text):
        if text not in english:
            raise ValueError(f"Add a reviewed English pronunciation: {text!r}")
        return english[text]
    model = KModel(repo_id=REPO_ID).to("cpu").eval()
    pipeline = KPipeline(lang_code="z", repo_id=REPO_ID, model=model, en_callable=en_callable)
    story_bytes = args.story.read_bytes()
    story = json.loads(story_bytes)
    timeline = np.zeros(round(story["duration"] * RATE), dtype=np.float32)
    reports = []
    overflow = []
    for scene in story["scenes"]:
        if args.scene and scene["id"] != args.scene:
            continue
        results = [result for result in pipeline(
            scene["caption"], voice=args.voice, speed=args.speed, split_pattern=r"[。；]"
        ) if result.audio is not None]
        pieces = [result.audio.numpy() for result in results]
        if not pieces:
            raise RuntimeError(f"No audio generated for {scene['id']}")
        gap = np.zeros(round(.12 * RATE), dtype=np.float32)
        samples = np.concatenate([part for i, p in enumerate(pieces) for part in ([gap, p] if i else [p])])
        duration = len(samples) / RATE
        available = scene["end"] - scene["start"] - .7
        sf.write(args.output / f"{scene['id']}.wav", samples, RATE, subtype="PCM_16")
        start = round((scene["start"] + .35) * RATE)
        cursor = .35
        caption_timings = []
        for result, piece in zip(results, pieces):
            caption_timings.append({"start": round(cursor, 3), "end": round(cursor + len(piece) / RATE, 3),
                                    "text": result.graphemes})
            cursor += len(piece) / RATE + .12
        report = {"id": scene["id"], "speech_seconds": round(duration, 3), "caption_timings_relative": caption_timings,
                  "available_seconds": available, "peak": float(np.abs(samples).max()),
                  "rms": float(np.sqrt(np.mean(samples**2))), "fits": duration <= available}
        print(json.dumps(report, ensure_ascii=False), flush=True)
        reports.append(report)
        if duration > available:
            overflow.append(scene["id"])
        else:
            timeline[start:start + len(samples)] += samples
    manifest = {"model": REPO_ID, "voice": args.voice, "speed": args.speed,
                "sample_rate": RATE, "story_sha256": hashlib.sha256(story_bytes).hexdigest(),
                "scenes": reports, "overflow": overflow}
    (args.output / "voice-report.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    if overflow:
        raise SystemExit("Revise narration or scene timing; no clipped final mix was written: " + ", ".join(overflow))
    if not args.scene:
        sf.write(args.output / "narration.wav", timeline, RATE, subtype="PCM_16")


if __name__ == "__main__":
    main()
