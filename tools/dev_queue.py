"""Persistent local verification queue. A running job owns a frozen source snapshot."""

from contextlib import closing
import json
import os
import sqlite3
import sys


def manage_queue(args, folder, root, detach, launch):
    folder.mkdir(parents=True, exist_ok=True)
    with closing(sqlite3.connect(folder / "queue.sqlite3", timeout=30)) as db:
        db.execute(
            "CREATE TABLE IF NOT EXISTS jobs ("
            "id INTEGER PRIMARY KEY, command TEXT NOT NULL, status TEXT NOT NULL, error TEXT)"
        )
        if args.queue_action == "worker":
            drain_queue(db, folder, launch)
            return
        if args.queue_action == "list":
            for job, command, status, error in db.execute(
                "SELECT id, command, status, error FROM jobs ORDER BY id"
            ):
                print(f"{job}	{status}	{json.loads(command)}")
                if status != "pending":
                    print(f"    {folder / str(job)}")
                if error:
                    print(f"    {error}")
            return
        command = json.dumps(args.job)
        with db:
            if args.queue_action == "add":
                job = db.execute(
                    "INSERT INTO jobs (command, status) VALUES (?, 'pending')", (command,)
                ).lastrowid
            else:
                job = args.id
                changed = db.execute(
                    "UPDATE jobs SET command = ? WHERE id = ? AND status = 'pending'",
                    (command, job),
                ).rowcount
                if not changed:
                    raise SystemExit(f"Job {job} must be pending to edit")
        detach(
            [sys.executable, "-u", root / "tools/dev.py", "queue", "worker"],
            root, folder / "worker.log", {**os.environ, "XIANXIA_RUNS": str(folder.parent)},
        )
        print(f"Job {job} {'queued' if args.queue_action == 'add' else 'updated'}")


def drain_queue(db, folder, launch):
    while True:
        with db:
            db.execute("BEGIN IMMEDIATE")
            entry = db.execute(
                "SELECT id, command FROM jobs WHERE status = 'pending' "
                "AND NOT EXISTS (SELECT 1 FROM jobs WHERE status = 'running') "
                "ORDER BY id LIMIT 1"
            ).fetchone()
            if entry is None:
                return
            job, command = entry
            db.execute("UPDATE jobs SET status = 'running' WHERE id = ?", (job,))
        run = folder / str(job)
        try:
            run.mkdir()
            status = launch(run, json.loads(command)).wait()
            error = f"Worker exited with status {status}" if status else None
        except Exception as exc:
            error = str(exc)
        with db:
            db.execute(
                "UPDATE jobs SET status = ?, error = ? WHERE id = ?",
                ("failed" if error else "passed", error, job),
            )
