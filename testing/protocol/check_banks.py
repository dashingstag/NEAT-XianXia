"""Check an isolated backend loaded with bank-demo.snapshot; never use the live port."""
import json
from check_protocol import send, snapshot

def check(stream):
    assert "leaderboard" not in snapshot(stream)
    send(stream, {"command":"watch_wealth","enabled":True})
    send(stream, {"command": "pause", "value": True})
    initial = snapshot(stream)
    assert initial["banking"]["loans"] and any(b["ready"] for b in initial["banks"])
    borrower = initial["banking"]["loans"][0]["person"]
    send(stream, {"command": "select", "agent": borrower})
    assert snapshot(stream)["selected"]["id"] == borrower
    for row in initial["leaderboard"]["rows"]:
        assert row["net_worth"] == row["coins"] + row["assets_value"] - row["debt"]
    send(stream, {"command": "save"})
    send(stream, {"command": "step", "ticks": 50})
    expected = snapshot(stream)
    send(stream, {"command": "load"})
    assert snapshot(stream)["leaderboard"] == initial["leaderboard"]
    send(stream, {"command": "step", "ticks": 50})
    restored = snapshot(stream)
    assert restored["banking"] == expected["banking"]
    assert restored["coins"] == expected["coins"]
    assert restored["leaderboard"] == expected["leaderboard"]
    send(stream, {"command":"watch_wealth","enabled":False})
    assert "leaderboard" not in snapshot(stream)
    print(json.dumps({"bank_protocol": "passed", "loan_replay": "passed", "active_loans": len(restored["banking"]["loans"])}))
