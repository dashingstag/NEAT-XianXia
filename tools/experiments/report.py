"""Offline benchmark comparison. Uses the Python standard library only."""
import argparse
import csv
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("reports", nargs="+", type=Path)
    parser.add_argument("--csv", type=Path)
    args = parser.parse_args()
    rows = []
    for path in args.reports:
        data = json.loads(path.read_text(encoding="utf-8"))
        world = data["world"]
        rows.append({"report": path.name, "population": world["allocated_agents"],
                     "living": world["living_agents"], "threads": data["threads"],
                     "mean_genes": round(data["gene_mean"], 2),
                     "seconds_simulated": data["simulated_seconds"],
                     "wall_seconds": round(data["step_wall_seconds"], 4),
                     "decisions_per_wall_second": round(data["decisions_per_wall_second"]),
                     "p95_tick_ms": round(data["p95_tick_ms"], 3),
                     "trades": world["stats"]["trades"], "shipments": world["stats"]["shipments"],
                     "accounting_ok": all(row["error"] == 0 for row in world["accounts"])
                     and world["coins"]["initial"] + world["coins"].get("minted",0) == world["coins"]["current"]})
    print(json.dumps(rows, indent=2))
    if args.csv:
        with args.csv.open("w", newline="", encoding="utf-8") as output:
            writer = csv.DictWriter(output, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)


if __name__ == "__main__":
    main()

