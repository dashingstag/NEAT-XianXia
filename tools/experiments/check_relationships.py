"""Exercise relationships on an isolated backend, and write an authentic UI fixture."""
import json
from pathlib import Path
from check_protocol import send, snapshot

def check(stream, out):
    send(stream, {"command": "pause", "value": True})
    send(stream, {"command": "step", "ticks": 1000})
    world = snapshot(stream)
    assert world["relationships"]["pairs"] > 0
    candidates = list(range(20)) + [c["leader"] for c in world["convoys"] if c.get("leader") is not None]
    chosen = None
    for person in dict.fromkeys(candidates):
        send(stream, {"command": "select", "agent": person})
        view = snapshot(stream)
        person_data = view["selected"]["person"]
        relationships = person_data["relationships"]
        assert len(relationships) <= 50
        assert all(-100 <= r["score"] <= 100 and r["score"] != 0 for r in relationships)
        if relationships:
            chosen = person
            peer = relationships[0]
            break
    assert chosen is not None, "No inspectable relationship was generated"
    send(stream, {"command": "select", "agent": peer["id"]})
    reciprocal = snapshot(stream)["selected"]["person"]["relationships"]
    assert any(r["id"] == chosen and r["score"] == peer["score"] for r in reciprocal)
    send(stream, {"command": "select", "agent": chosen})
    send(stream, {"command": "save"})
    send(stream, {"command": "step", "ticks": 50})
    expected = snapshot(stream)
    send(stream, {"command": "load"})
    send(stream, {"command": "select", "agent": chosen})
    send(stream, {"command": "step", "ticks": 50})
    restored = snapshot(stream)
    assert restored["relationships"] == expected["relationships"]
    assert restored["selected"]["person"]["relationships"] == expected["selected"]["person"]["relationships"]
    assert restored["coins"] == expected["coins"]
    assert restored["banking"] == expected["banking"]
    Path(out).write_text(json.dumps({"world": restored, "business_candidates": [restored["selected"]]}), encoding="utf-8")
    print(json.dumps({"relationship_protocol": "passed", "reciprocal_scores": "passed", "replay": "passed", "pairs": restored["relationships"]["pairs"], "selected_person": chosen}))
