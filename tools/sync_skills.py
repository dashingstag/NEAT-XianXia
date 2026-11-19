"""Publish project skills to client discovery folders and check tracked copies."""
import argparse
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    copies = []
    for skill in sorted((ROOT / "skills").glob("*/SKILL.md")):
        for base in [".agents/skills", ".github/skills"]:
            copies.append((skill, ROOT / base / skill.parent.name / skill.name))
    agent = ROOT / "third_party/awesome-copilot/agents/tdd-red.agent.md"
    copies.append((agent, ROOT / ".github/agents" / agent.name))
    for source, target in copies:
        if args.check:
            if not target.exists() or source.read_text(encoding="utf-8") != target.read_text(encoding="utf-8"):
                raise SystemExit(f"Refresh discovery copy: {target.relative_to(ROOT)}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
    print(f"{len(copies)} discovery files {'verified' if args.check else 'synchronized'}")


if __name__ == "__main__":
    main()
