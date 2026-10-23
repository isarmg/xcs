"""Resolve grouped foundation sources and rebase isolated consumer dependencies.

External test trees retain their existing locations. Only dependency paths are
made absolute when copying a package, so source/test path fields still exercise
the copied tree at its original repository-relative depth.
"""
from pathlib import Path
import json
import re
import tomllib


def package_path(root, relative):
    """Find a package beneath one functional group without hiding duplicates.

    Missing/retired roots retain their flat spelling for negative existence
    checks. This also lets small synthetic fixtures describe an absent package.
    """
    base = Path(root) / 'src/foundation'
    relative = Path(relative)
    if relative.is_absolute() or '..' in relative.parts or not relative.parts:
        raise ValueError(f'invalid foundation-relative path: {relative}')
    name, *tail = relative.parts
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*', name):
        raise ValueError(f'invalid foundation package name: {name}')
    candidates = [path for path in (base / name, *(group / name for group in base.glob('*')))
                  if path.is_dir()]
    if len(candidates) > 1:
        raise ValueError(f'ambiguous foundation package {name}: {candidates}')
    package = candidates[0] if candidates else base / name
    return package.joinpath(*tail)


def rebase_dependency_paths(manifest, source_package):
    """Rebase dependency fields only, preserving source/test paths and comments."""
    # Validate the complete input before making even a temporary fixture edit.
    tomllib.loads(manifest)
    source_package = Path(source_package)
    dependency_sections = {'dependencies', 'dev-dependencies', 'build-dependencies'}
    section = ()
    output = []

    def section_path(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key == '__foundation_path_probe__':
                    return ()
                found = section_path(child)
                if found is not None:
                    return (key, *found)
        elif isinstance(value, list):
            for child in value:
                found = section_path(child)
                if found is not None:
                    return found
        return None

    def without_comment(line):
        # A # inside a TOML string is data, not the start of a comment.
        quote = None
        escaped = False
        for index, character in enumerate(line):
            if quote is not None:
                if quote == '"' and not escaped and character == "\\":
                    escaped = True
                    continue
                if not escaped and character == quote:
                    quote = None
                escaped = False
            elif character in ('"', "'"):
                quote = character
            elif character == '#':
                return line[:index], line[index:]
        return line, ''

    path_assignment = re.compile(r"(?P<prefix>(?:^|[,{])\s*path\s*=\s*)(?P<value>\"(?:\\.|[^\"\\])*\"|'[^']*')")
    for line in manifest.splitlines(keepends=True):
        code, comment = without_comment(line)
        stripped = code.strip()
        if stripped.startswith('['):
            # Let TOML decode quoted cfg/table keys instead of splitting on dots.
            section = section_path(tomllib.loads(stripped + '\n__foundation_path_probe__ = true\n'))
        dependency_table = (
            len(section) in (1, 2) and section[0] in dependency_sections
            or len(section) in (3, 4) and section[0] == 'target' and section[2] in dependency_sections
            or len(section) in (2, 3) and section[:2] == ('workspace', 'dependencies')
        )
        if dependency_table:
            def replace(match):
                path = tomllib.loads('path = ' + match['value'])['path']
                return match['prefix'] + json.dumps(str((source_package / path).resolve()))
            code = path_assignment.sub(replace, code)
        output.append(code + comment)
    return ''.join(output)
