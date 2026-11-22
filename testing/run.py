#!/usr/bin/env python3
"""Single entry point for the repository's verification and performance tools."""
from pathlib import Path
import sys


if __name__ == "__main__":
    tools = Path(__file__).resolve().parents[1] / "tools"
    sys.path.insert(0, str(tools))
    from dev import main

    main()
