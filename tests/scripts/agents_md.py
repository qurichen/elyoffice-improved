#!/usr/bin/env python3
"""agents.md names resolve to public items in the library."""
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
lib = root / "reference/Ely-GPUI-Components/src"
ITEM = re.compile(r"^pub\s+(?:struct|enum|fn|trait|type|const)\s+([A-Za-z_]\w*)", re.M)
USE = re.compile(r"pub\s+use\s+([^;]+);")
CLAIM = re.compile(r"([a-z_]+)::\{([^}]*)\}")


def public_names(module: str) -> set[str]:
    mod_rs = lib / module / "mod.rs"
    if not mod_rs.is_file():
        sys.exit(f"FAIL: no module {module} in the component library")
    text = mod_rs.read_text()
    names = set(ITEM.findall(text))
    for use in USE.findall(text):
        use = " ".join(use.split())
        if use.endswith("::*"):
            names |= glob_names(module, use[:-3])
            continue
        group = re.match(r".+::\{(.*)\}$", use)
        parts = group.group(1).split(",") if group else [use]
        names |= {p.split(" as ")[-1].rsplit("::", 1)[-1].strip() for p in parts if p.strip()}
    return names


def glob_names(module: str, sub: str) -> set[str]:
    for path in (lib / module / f"{sub}.rs", lib / module / sub / "mod.rs"):
        if path.is_file():
            return set(ITEM.findall(path.read_text()))
    sys.exit(f"FAIL: cannot resolve {module}: pub use {sub}::*")


count = 0
for module, listed in CLAIM.findall((root / "agents.md").read_text()):
    public = public_names(module)
    for name in (n.strip() for n in listed.split(",")):
        if name not in public:
            sys.exit(f"FAIL: {module}::{name} is not public in the component library")
        count += 1

if count == 0:
    sys.exit("FAIL: agents.md names no components")
print(f"agents.md: {count} component names are public")
