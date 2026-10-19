#!/usr/bin/env python3
"""Trim only low-energy sentence edges in v2 auditions; retain interior pauses."""
import argparse
import json
from pathlib import Path


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--directory", type=Path, required=True)
    args = p.parse_args()
    import numpy as np
    import soundfile as sf
    report = json.loads((args.directory / "audition-report.json").read_text())
    threshold = 10 ** (-50 / 20)
    candidates = []
    for candidate in report["candidates"]:
        samples, rate = sf.read(args.directory / candidate["file"], dtype="float32")
        frame = round(.01 * rate)
        keep = round(.06 * rate)
        def energy_stats(audio):
            frames = len(audio) // frame
            rms = np.sqrt(np.mean(audio[:frames * frame].reshape(-1, frame) ** 2, axis=1))
            quiet = int(np.sum(rms <= threshold)) * frame / rate
            duration = len(audio) / rate
            return {"duration": duration, "low_energy_seconds": quiet,
                    "low_energy_fraction": quiet / duration,
                    "above_threshold_fraction": 1 - quiet / duration}
        pieces, timings, audit = [], [], []
        cursor = 0
        for item in candidate["timings"]:
            chunk = samples[round(item["start"] * rate):round(item["end"] * rate)]
            frames = len(chunk) // frame
            rms = np.sqrt(np.mean(chunk[:frames * frame].reshape(-1, frame) ** 2, axis=1))
            active = np.flatnonzero(rms > threshold)
            if not len(active):
                raise ValueError("Refusing to trim a sentence with no detectable signal")
            start = max(0, int(active[0]) * frame - keep)
            end = min(len(chunk), (int(active[-1]) + 1) * frame + keep)
            trimmed = chunk[start:end]
            if pieces:
                gap = np.zeros(keep, dtype=np.float32)
                pieces.append(gap)
                cursor += len(gap)
            timings.append({"index": item["index"], "text": item["text"],
                            "start": cursor / rate, "end": (cursor + len(trimmed)) / rate})
            audit.append({"index": item["index"], "removed_head_seconds": start / rate,
                          "removed_tail_seconds": (len(chunk) - end) / rate})
            pieces.append(trimmed)
            cursor += len(trimmed)
        result = np.concatenate(pieces)
        filename = f"audition-{candidate['voice']}-trimmed.wav"
        sf.write(args.directory / filename, result, rate, subtype="PCM_16")
        updated = {**candidate, "file": filename, "duration": len(result) / rate,
                   "timings": timings, "edge_trims": audit,
                   "before": energy_stats(samples), "after": energy_stats(result)}
        candidates.append(updated)
        print(json.dumps({"voice": candidate["voice"], "before": updated["before"],
                          "after": updated["after"]}, ensure_ascii=False), flush=True)
    report.update({"candidates": candidates, "trim_method": {
        "rms_threshold_dbfs": -50, "frame_seconds": .01, "retain_each_edge_seconds": .06,
        "inter_sentence_gap_seconds": .06, "interior_pauses": "unchanged",
        "statistics_note": "Energy threshold statistics, not linguistic speech detection"}})
    (args.directory / "audition-trimmed-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
