"""Inventory simple manifest-relative paths in declared test/bench entrypoints.

This is a deliberately finite syntax check, not a Rust parser or a module-graph
analysis. It recognizes literal concat!(env!(...), suffix) and
Path::new/PathBuf::from(env!(...)).join(suffix) expressions. Dynamic expressions,
implicit Cargo targets and references in nested modules are outside its scope.
"""
import json
from pathlib import Path
import re
import tomllib

_ENV = r'(?:core::)?env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)'
_LITERAL = r'"([^"\\\r\n]*)"'
_PATTERNS = (
    ('concat', re.compile(r'(?<![\w:])(?:core::)?concat!\s*\(\s*' + _ENV
                          + r'\s*,\s*' + _LITERAL + r'\s*,?\s*\)')),
    ('join', re.compile(r'(?<![\w:])(?:std::path::)?(?:Path::new|PathBuf::from)\s*\(\s*'
                        + _ENV + r'\s*\)\s*\.\s*join\s*\(\s*'
                        + _LITERAL + r'\s*,?\s*\)')),
)


def _code_positions(text):
    """Exclude comments and quoted literals from recognized expression starts."""
    code = bytearray(b'\1') * len(text)
    raw_pattern = re.compile(r'(?:b|c)?r(#+)?"')
    char_pattern = re.compile(r"'(?:\\.|[^'\\\n])'")
    index = 0
    while index < len(text):
        end = None
        if text.startswith('//', index):
            end = text.find('\n', index)
            if end < 0:
                end = len(text)
        elif text.startswith('/*', index):
            depth, end = 1, index + 2
            while end < len(text) and depth:
                if text.startswith('/*', end):
                    depth += 1
                    end += 2
                elif text.startswith('*/', end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        else:
            raw = raw_pattern.match(text, index)
            if raw:
                closing = '"' + (raw[1] or '')
                position = text.find(closing, raw.end())
                end = len(text) if position < 0 else position + len(closing)
            elif text[index] == '"':
                end = index + 1
                while end < len(text):
                    if text[end] == '\\':
                        end += 2
                    elif text[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
            elif text[index] == "'":
                char = char_pattern.match(text, index)
                if char:
                    end = char.end()
        if end is None:
            index += 1
        else:
            code[index:end] = b'\0' * (end - index)
            index = end
    return code


def references(text):
    """Return only the two documented static forms, in source order."""
    code = _code_positions(text)
    found = [(match.start(), form, match[1])
             for form, pattern in _PATTERNS for match in pattern.finditer(text)
             if code[match.start()]]
    return [(form, literal) for _, form, literal in sorted(found)]


def declared_targets(root):
    """Use current catalog owners and actual root-workspace manifests."""
    root = Path(root).resolve()
    catalog = json.loads((root / 'compliance/rust-sources.json').read_text())
    workspace = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']
    manifests = {root / owner / 'Cargo.toml' for owner in catalog['packages']}
    for member in workspace['members']:
        manifests.update(path / 'Cargo.toml' for path in root.glob(member))
    targets = []
    for manifest in sorted(manifests):
        data = tomllib.loads(manifest.read_text())
        for kind in ('test', 'bench'):
            for target in data.get(kind, []):
                if 'path' in target:
                    source = manifest.parent / target['path']
                else:
                    base = manifest.parent / (kind + 's') / target['name']
                    choices = [path for path in (Path(str(base) + '.rs'), base / 'main.rs')
                               if path.is_file()]
                    if len(choices) != 1:
                        raise ValueError(f'ambiguous or missing declared target: {manifest}: {target}')
                    source = choices[0]
                source = source.resolve()
                if not source.is_file():
                    raise ValueError(f'missing declared target: {source}')
                targets.append({'manifest': manifest.relative_to(root).as_posix(),
                                'kind': kind, 'name': target['name'],
                                'source': source.relative_to(root).as_posix()})
    return targets


def resolve_reference(manifest, form, literal):
    owner = Path(manifest).parent
    if form == 'concat':
        return Path(str(owner) + literal).resolve()
    if form == 'join':
        return (owner / literal).resolve()
    raise ValueError('unrecognized manifest-relative path form: ' + form)


def inventory(root):
    root = Path(root).resolve()
    rows = []
    for target in declared_targets(root):
        for form, literal in references((root / target['source']).read_text()):
            resolved = resolve_reference(root / target['manifest'], form, literal)
            if not resolved.exists():
                raise ValueError(f'missing manifest-relative input: {target}: {literal}')
            rows.append(dict(target, form=form, literal=literal,
                             destination=resolved.relative_to(root).as_posix(),
                             destination_kind='directory' if resolved.is_dir() else 'file'))
    return rows
