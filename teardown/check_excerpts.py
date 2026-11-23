"""Fail when a teardown excerpt no longer matches the file it quotes.

Every excerpt in teardown/audit.json is {"file", "s", "e", "text"}. The text
must equal lines s..e (1-based, inclusive) of the file at the current
revision, ignoring trailing whitespace and leading indentation, so a stale
audit cannot cite code that has moved or changed. A line that is exactly
"..." elides any number of lines; the pieces around it must appear in order.

    python teardown/check_excerpts.py            # report and exit 1 on drift
    python teardown/check_excerpts.py --locate   # also say where moved text now is
    python teardown/check_excerpts.py --fix      # rewrite line numbers of moved excerpts
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def norm(lines):
    return [line.strip() for line in lines]


def excerpts(node, part=None):
    if isinstance(node, dict):
        part = node.get("id", part)
        if {"file", "s", "e", "text"} <= node.keys():
            yield part, node
        for value in node.values():
            yield from excerpts(value, part)
    elif isinstance(node, list):
        for value in node:
            yield from excerpts(value, part)


def matches(lines, want):
    """True when `want` (with optional "..." elisions) spans exactly `lines`."""
    pieces, piece = [], []
    for line in want:
        if line == "...":
            pieces.append(piece)
            piece = []
        else:
            piece.append(line)
    pieces.append(piece)
    if len(pieces) == 1:
        return lines == want
    if lines[:len(pieces[0])] != pieces[0] or lines[len(lines) - len(pieces[-1]):] != pieces[-1]:
        return False
    at = len(pieces[0])
    for piece in pieces[1:-1]:
        found = locate(lines[at:len(lines) - len(pieces[-1])], piece)
        if found is None:
            return False
        at += found - 1 + len(piece)
    return True


def locate(lines, want):
    """Return the 1-based line where `want` now starts, or None."""
    for start in range(len(lines) - len(want) + 1):
        if lines[start:start + len(want)] == want:
            return start + 1
    return None


def main():
    source = ROOT / "teardown/audit.json"
    raw = source.read_text(encoding="utf-8")
    audit = json.loads(raw)
    fix = "--fix" in sys.argv
    show = fix or "--locate" in sys.argv
    total, stale = 0, []
    for part, ex in excerpts(audit):
        total += 1
        path = ROOT / ex["file"]
        if not path.is_file():
            stale.append((part, ex, "file missing"))
            continue
        lines = norm(path.read_text(encoding="utf-8", errors="replace").splitlines())
        want = norm(ex["text"].splitlines())
        if matches(lines[ex["s"] - 1:ex["e"]], want):
            continue
        moved = locate(lines, want) if show and "..." not in want else None
        if fix and moved:
            # Patch the one-line excerpt object in place to keep the diff small.
            old = f'"file": {json.dumps(ex["file"])}, "s": {ex["s"]}, "e": {ex["e"]},'
            new = f'"file": {json.dumps(ex["file"])}, "s": {moved}, "e": {moved + len(want) - 1},'
            if raw.count(old) == 1:
                raw = raw.replace(old, new)
                continue
        stale.append((part, ex, f"moved to line {moved}" if moved else "text not found"))
    if fix:
        source.write_text(raw, encoding="utf-8")
    for part, ex, why in stale:
        print(f"{part}: {ex['file']}:{ex['s']}-{ex['e']} {why}")
    print(f"{total - len(stale)}/{total} excerpts match the current source")
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
