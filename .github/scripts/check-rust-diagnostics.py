#!/usr/bin/env python3
"""Run a Cargo JSON build and reject Rust diagnostics from every local package."""
import argparse
import json
import subprocess
import sys


def invalid_constant(value):
    raise ValueError(f"not a JSON constant: {value}")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


class CargoDiagnostics:
    """Validate the build stream independently of Cargo's process exit status."""

    def __init__(self, output):
        self.output = output
        self.finished = False
        self.succeeded = False
        self.warnings = 0
        self.errors = 0
        self.invalid = False

    def reject(self, explanation):
        self.invalid = True
        print(f"[rust-diagnostics] {explanation}", file=self.output, flush=True)

    def consume(self, line, number):
        try:
            event = json.loads(line, parse_constant=invalid_constant,
                               object_pairs_hook=unique_object)
        except (ValueError, TypeError):
            self.reject(f"invalid Cargo JSON on line {number}: {line.rstrip()}")
            return
        if not isinstance(event, dict):
            self.reject(f"Cargo message on line {number} must be an object")
            return
        if self.finished:
            self.reject(f"Cargo message after build-finished on line {number}")
        reason = event.get("reason")
        if reason == "compiler-message":
            message = event.get("message")
            if not isinstance(message, dict) or not isinstance(message.get("message"), str):
                self.reject(f"invalid compiler-message on line {number}")
                return
            level = message.get("level")
            if level == "warning":
                self.warnings += 1
            elif level in ("error", "failure-note", "ice"):
                self.errors += 1
            elif level not in ("note", "help"):
                self.reject(f"unknown compiler diagnostic level on line {number}: {level!r}")
            rendered = message.get("rendered")
            if rendered is not None and not isinstance(rendered, str):
                self.reject(f"invalid rendered diagnostic on line {number}")
                return
            print(rendered or f"{level}: {message['message']}",
                  file=self.output, end="" if rendered and rendered.endswith("\n") else "\n",
                  flush=True)
        elif reason == "build-finished":
            self.finished = True
            if type(event.get("success")) is not bool:
                self.reject(f"invalid build-finished success on line {number}")
            else:
                self.succeeded = event["success"]
        elif reason not in ("compiler-artifact", "build-script-executed"):
            self.reject(f"unknown Cargo message reason on line {number}: {reason!r}")

    def result(self, returncode):
        if not self.finished:
            self.reject("Cargo did not emit build-finished")
        if returncode != 0 or not self.succeeded or self.invalid or self.warnings or self.errors:
            print("[rust-diagnostics] build rejected: "
                  f"Cargo exit {returncode}, success={self.succeeded}, "
                  f"{self.warnings} Rust warning(s), {self.errors} Rust error(s), "
                  f"invalid stream={self.invalid}", file=self.output, flush=True)
            return returncode if returncode > 0 else 1
        return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs=argparse.REMAINDER,
                        help="-- cargo check/build/clippy/test --no-run --message-format=json ...")
    command = parser.parse_args().command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        parser.error("a Cargo build command is required")
    diagnostics = CargoDiagnostics(sys.stderr)
    try:
        # Cargo progress and build-script output remain on stderr. Only Cargo's
        # JSON stdout is decoded; rendered Rust diagnostics are printed above.
        with subprocess.Popen(command, stdout=subprocess.PIPE, text=True,
                              encoding="utf-8") as process:
            try:
                for number, line in enumerate(process.stdout, 1):
                    diagnostics.consume(line, number)
            except UnicodeError:
                diagnostics.reject("Cargo stdout is not valid UTF-8")
                process.terminate()
            except KeyboardInterrupt:
                process.terminate()
                return 130
            return diagnostics.result(process.wait())
    except OSError as error:
        print(f"[rust-diagnostics] could not run Cargo command: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
