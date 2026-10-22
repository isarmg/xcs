"""Resolve local Cargo dependency declarations without losing consumer options."""
from pathlib import Path


def resolve(root, relative, manifest, workspace, alias, declaration):
    """Return a fresh dependency specification and the base of its source path.

    Cargo 1.99 adds workspace and consumer features, keeps optionality local,
    and lets edition-2024 consumers override inherited default features. Older
    editions cannot disable defaults that the workspace enables. This does not
    select features: dormant declarations still need independent validation.
    """
    if not isinstance(declaration, dict):
        raise ValueError(f"non-path dependency: {relative} {alias}")
    if declaration.get("workspace") is not True:
        return dict(declaration), root / Path(relative).parent
    inherited = workspace.get("dependencies", {}).get(alias)
    if not isinstance(inherited, dict):
        raise ValueError(f"non-path dependency: {relative} {alias}")
    dependency = dict(inherited)
    dependency["features"] = [*inherited.get("features", []), *declaration.get("features", [])]
    dependency["optional"] = declaration.get("optional", False)
    dependency["default-features"] = declaration.get("default-features", inherited.get("default-features", True))
    edition = manifest.get("package", {}).get("edition", "2015")
    if isinstance(edition, dict) and edition.get("workspace") is True:
        edition = workspace.get("package", {}).get("edition", "2015")
    if int(edition) < 2024 and inherited.get("default-features", True):
        dependency["default-features"] = True
    return dependency, root
