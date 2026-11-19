"""Local entry point shared by contributors, skills and CI. Requires Python 3.11+."""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CONFIG = tomllib.loads((ROOT / "project.toml").read_text())
SCENARIOS = tomllib.loads((ROOT / CONFIG["project"]["world"]).read_text())
CONTRACT = CONFIG["contracts"]
sys.path.insert(0, str(ROOT / "tools/experiments"))
FLAGS = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0


def record(path, data):
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def digest(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def cargo(*args):
    subprocess.run(["cargo", *args, "--locked", "--manifest-path",
                    ROOT / CONFIG["project"]["sim"]], cwd=ROOT, check=True)


def backend():
    name = "sim-runner.exe" if os.name == "nt" else "sim-runner"
    bundled = ROOT / "bin" / name
    if bundled.exists():
        return bundled
    cargo("build", "--release")
    return (Path(os.getenv("CARGO_TARGET_DIR", ROOT / "sim/target")) / "release" / name).resolve()


def godot():
    configured = os.getenv("GODOT_BIN") or shutil.which("godot")
    if configured:
        return configured
    if os.name == "nt":
        filename = f"Godot_v{CONFIG['toolchain']['godot']}-stable_win64.exe"
        downloads = Path.home() / "Downloads"
        for candidate in (downloads / filename, downloads / filename / filename):
            if candidate.is_file():
                return candidate
    return "godot"


@contextmanager
def server(binary, folder, checkpoint=None, scenario="protocol"):
    folder.mkdir(parents=True, exist_ok=True)
    ready = folder / "ready.json"
    save = checkpoint or folder / "world.snapshot"
    settings = SCENARIOS[scenario]
    command = [binary, "serve", "--port", "0", "--agents", str(settings["agents"]),
               "--seed", str(settings["seed"]), "--threads", "2", "--checkpoint", save]
    if settings.get("map"):
        command += ["--map", str(settings["map"])]
    if save.is_file():
        command.append("--load")
    with (folder / "backend.log").open("w") as log:
        proc = subprocess.Popen(command, cwd=folder, stdout=log, stderr=subprocess.STDOUT,
                                env={**os.environ, "XIANXIA_READY_FILE": str(ready)},
                                creationflags=FLAGS)
        try:
            deadline = time.monotonic() + 90
            while not ready.exists():
                if proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f"Backend startup failed. See {log.name}")
                time.sleep(0.05)
            info = json.loads(ready.read_text())
            if info["pid"] != proc.pid or proc.poll() is not None:
                raise RuntimeError("Unexpected backend readiness record")
            yield info["port"]
        finally:
            if proc.poll() is None:
                proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()


def connect(port):
    return socket.create_connection(("127.0.0.1", port), timeout=60)


def protocol_suite(run):
    import check_banks
    import check_protocol
    import check_relationships
    binary = backend()
    with server(binary, run / "fresh") as port, connect(port) as stream:
        check_protocol.check(stream)
    for name, check in [("bank", check_banks.check), ("relationships", check_relationships.check)]:
        folder = run / name
        folder.mkdir()
        save = folder / "world.snapshot"
        shutil.copyfile(ROOT / CONTRACT["old_save"], save)
        with server(binary, folder, save) as port, connect(port) as stream:
            if name == "relationships":
                check(stream, run / "town-shop.json")
            else:
                check(stream)


def isolation(run):
    from check_protocol import send, snapshot
    binary = backend()
    with server(binary, run / "survivor") as first, connect(first) as survivor:
        send(survivor, {"command": "reset", "agents": 80, "seed": 11})
        send(survivor, {"command": "save"})
        before = digest(run / "survivor/world.snapshot")
        class DeliberateFailure(Exception):
            pass
        try:
            with server(binary, run / "interrupted") as second, connect(second) as peer:
                assert first != second
                send(peer, {"command": "reset", "agents": 96, "seed": 22})
                send(peer, {"command": "save"})
                assert snapshot(peer)["seed"] == 22
                raise DeliberateFailure()
        except DeliberateFailure:
            pass
        with socket.socket() as probe:
            probe.settimeout(2)
            assert probe.connect_ex(("127.0.0.1", second)) != 0
        send(survivor, {"command": "load"})
        assert snapshot(survivor)["seed"] == 11
        assert digest(run / "survivor/world.snapshot") == before
        send(survivor, {"command": "step", "ticks": 1})
        assert snapshot(survivor)["tick"] == 1
        record(run / "isolation.json", {"ports": [first, second], "surviving_save_sha256": before})


def replay(run):
    from check_protocol import send
    save = run / "world.snapshot"
    shutil.copyfile(ROOT / CONTRACT["old_save"], save)
    with server(backend(), run, save) as port, connect(port) as stream:
        send(stream, {"command": "step", "ticks": 25})
        send(stream, {"command": "save"})
        midpoint = run / "midpoint.snapshot"
        shutil.copyfile(save, midpoint)
        send(stream, {"command": "step", "ticks": 50})
        send(stream, {"command": "save"})
        continuous = digest(save)
        shutil.copyfile(midpoint, save)
        send(stream, {"command": "load"})
        send(stream, {"command": "step", "ticks": 50})
        send(stream, {"command": "save"})
        resumed = digest(save)
        record(run / "replay.json", {"continuous": continuous, "resumed": resumed, "ticks": 50})
        assert continuous == resumed, "Serialized worlds diverged after replay"


def viewer(run, preview=False):
    command = [godot(), "--path", ROOT / CONFIG["project"]["viewer"], "--log-file", run / "godot.log"]
    if not preview:
        command += ["--headless", "--quit-after", "150"]
    command += ["--script", "res://bench/urban_ui_check.gd", "--", ROOT / CONTRACT["town_frame"]]
    if preview:
        command.append("--preview")
    result = subprocess.run(command, capture_output=True, text=True, errors="replace", check=False)
    output = result.stdout + result.stderr
    (run / "viewer.txt").write_text(output, encoding="utf-8")
    if result.returncode or "SCRIPT ERROR" in output or (not preview and '"urban_ui":"passed"' not in output):
        raise RuntimeError(f"Viewer check failed. See {run / 'viewer.txt'}")


def labs():
    for folder in sorted((ROOT / "labs").iterdir()):
        subprocess.run(["node", "--test", *sorted(folder.glob("*.tests.mjs")), folder / "tests.mjs"],
                       cwd=folder, check=True)


def connection_check(run):
    with server(backend(), run / "connected-viewer") as port:
        result = subprocess.run([godot(), "--headless", "--path", ROOT / "viewer", "--log-file",
                                 run / "connected-godot.log", "--script", "res://bench/connection_check.gd"],
                                env={**os.environ, "XIANXIA_PORT": str(port)}, capture_output=True,
                                text=True, errors="replace", timeout=35)
        output = result.stdout + result.stderr
        (run / "connected-viewer.txt").write_text(output, encoding="utf-8")
        if result.returncode or "SCRIPT ERROR" in output or "Verified connected Godot viewer" not in output:
            raise RuntimeError(f"Live viewer smoke check failed. See {run / 'connected-viewer.txt'}")


@contextmanager
def player_lock(folder):
    # OS releases the lock when the process exits, including after a crash.
    with (folder / "session.lock").open("a+b") as lock:
        if os.name == "nt":
            import msvcrt
            lock.seek(0)
            msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def play(run, scenario="founders", fresh=False):
    folder = run / "player" if fresh else Path(
        os.getenv("XIANXIA_PLAYER_DIR", Path.home() / ".xianxia-handoff")
    ).resolve()
    folder.mkdir(parents=True, exist_ok=True)
    if fresh:
        print(f"Fresh game profile: {folder}", flush=True)
    with player_lock(folder):
        binary = Path(shutil.copy2(backend(), run))
        with server(binary, run, folder / "world.snapshot", scenario) as port:
            subprocess.run([godot(), "--path", ROOT / "viewer", "--log-file", run / "viewer.log"],
                           env={**os.environ, "XIANXIA_PORT": str(port)}, check=True)


def package(run):
    stage = run / "xianxia"
    (stage / "bin").mkdir(parents=True)
    shutil.copy2(backend(), stage / "bin")
    for name in ["viewer", "tools", "world", "fixtures"]:
        shutil.copytree(ROOT / name, stage / name, ignore=shutil.ignore_patterns(".godot", "__pycache__"))
    for name in ["project.toml", "README.md", "LICENSE"]:
        shutil.copy2(ROOT / name, stage)
    shutil.copy2(ROOT / "docs/PLAY.md", stage / "PLAY.md")
    record(stage / "release.json", {"contracts": CONTRACT, "toolchain": CONFIG["toolchain"],
           "platform": platform.platform(), "backend_sha256": digest(next((stage / "bin").iterdir()))})
    print(shutil.make_archive(str(run / "xianxia"), "zip", run, "xianxia"))


def doctor():
    for name, program, expected in [("Rust", "rustc", CONFIG["toolchain"]["rust"]),
                                    ("Godot", godot(), CONFIG["toolchain"]["godot"]),
                                    ("Node", "node", str(CONFIG["toolchain"]["node_major"]))]:
        result = subprocess.check_output([program, "--version"], text=True).strip()
        print(f"{name}: {result}")
        version = result.removeprefix("rustc ").removeprefix("v").split()[0]
        if version != expected and not version.startswith(expected + "."):
            raise RuntimeError(f"Expected {name} {expected}. Set GODOT_BIN for a custom Godot path.")
    print("Python:", sys.version.split()[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    tests = sub.add_parser("test")
    tests.add_argument("suite", choices=["sim", "viewer", "connection", "protocol", "replay", "isolation", "labs", "world", "all"])
    for name in ["doctor", "package"]:
        sub.add_parser(name)
    play_command = sub.add_parser("play")
    play_command.add_argument("--fresh", action="store_true",
                              help="Start with a new profile in this run's evidence folder")
    play_command.add_argument(
        "--scenario", default="founders",
        choices=[k for k, v in SCENARIOS.items() if "agents" in v])
    sub.add_parser("preview").add_argument("--fixture", choices=["town-shop"], default="town-shop")
    sub.add_parser("bench").add_argument("--scenario", choices=["mature-town"], default="mature-town")
    args = parser.parse_args()
    if not __debug__:
        parser.error("Run without Python optimization so assertions are active")
    base = Path(os.getenv("XIANXIA_RUNS", ROOT / ".work")).resolve()
    base.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=args.command + "-", dir=base))
    print(f"Run evidence: {run}", flush=True)
    try:
        if args.command == "test":
            suites = {"sim": lambda: cargo("test", "--workspace"), "labs": labs,
                      "world": lambda: cargo("test", "-p", "sim-core", "landscape"),
                      "viewer": lambda: viewer(run), "connection": lambda: connection_check(run),
                      "isolation": lambda: isolation(run),
                      "replay": lambda: replay(run), "protocol": lambda: protocol_suite(run)}
            for name in suites if args.suite == "all" else [args.suite]:
                suites[name]()
        elif args.command == "bench":
            scenario = SCENARIOS[args.scenario]
            with (run / "benchmark.log").open("w") as log:
                subprocess.run([backend(), "bench", "--load", "--checkpoint", ROOT / scenario["checkpoint"],
                                "--steps", str(scenario["steps"]), "--out", run / "benchmark.json"],
                               stdout=log, check=True)
        elif args.command == "doctor":
            doctor()
        elif args.command == "preview":
            viewer(run, preview=True)
        elif args.command == "package":
            package(run)
        else:
            play(run, scenario=args.scenario, fresh=args.fresh)
    except Exception as error:
        record(run / "result.json", {"args": vars(args), "passed": False, "error": repr(error)})
        raise
    record(run / "result.json", {"args": vars(args), "passed": True})


if __name__ == "__main__":
    main()
