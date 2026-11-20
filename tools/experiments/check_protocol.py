"""Exercise the actual native TCP API; leave the world paused after verification."""
import base64
import json
import struct


def exact(stream, length):
    result = bytearray()
    while len(result) < length:
        part = stream.recv(length - len(result))
        if not part:
            raise RuntimeError("Backend disconnected")
        result.extend(part)
    return bytes(result)


def frame(stream):
    length, = struct.unpack("<I", exact(stream, 4))
    assert 4 <= length <= 4_194_304
    return exact(stream, length)


def send(stream, command):
    data = json.dumps(command).encode()
    stream.sendall(struct.pack("<I", len(data)) + data)
    for _ in range(30):
        message = frame(stream)
        if message[:4] == b"XNC1":
            result = json.loads(message[4:])
            assert result["ok"], result
            return result
    raise AssertionError("No command reply")


def snapshot(stream):
    for _ in range(30):
        data = frame(stream)
        if data[:4] != b"XNS1":
            continue
        version, flags, tick, agents, visible, meta_len, animals = struct.unpack_from("<HHQIIII", data, 4)
        assert version == 2 and len(data) == 32 + meta_len + (visible + animals) * 16
        meta = json.loads(data[32:32 + meta_len])
        at = 32 + meta_len + visible * 16
        meta["wildlife"]["animals"] = [
            dict(zip(("id", "x", "y", "species", "action", "health_young", "heading"),
                     struct.unpack_from("<IffBBBB", data, at + i * 16)))
            for i in range(animals)
        ]
        meta["paused"] = bool(flags & 1)
        assert all(row["error"] == 0 for row in meta["accounts"])
        assert meta["wildlife"]["grass_account"]["error"] == 0
        assert all(row["error"]==0 for row in meta["urban"]["ware_accounts"])
        assert meta["coins"]["initial"] + meta["coins"].get("minted",0) == meta["coins"]["current"]
        return meta
    raise AssertionError("No snapshot")


def check(stream):
    send(stream, {"command": "pause", "value": True})
    send(stream, {"command": "reset", "agents": 1000, "seed": 77})
    send(stream, {"command": "step", "ticks": 120})
    first = snapshot(stream)
    assert "schools" in first and "education" in first
    assert "taverns" in first and 0 <= first["selected"]["person"]["social_score"] <= 100
    assert first["map_width"] == 3072 and first["map_height"] == 1920
    layers = send(stream, {"command": "map_layers"})["map_layers"]
    assert layers["map"]["size"] == "Classic"
    terrain, explored = layers["terrain"], layers["explored"]
    assert (terrain["cols"], terrain["rows"], terrain["cell"]) == (64, 40, 48.0)
    assert all(len(base64.b64decode(terrain[k])) == 64 * 40 for k in ("classes", "elevation", "qi"))
    assert max(base64.b64decode(terrain["classes"])) <= 5
    known = base64.b64decode(explored["known"])
    assert (explored["cols"], explored["rows"], explored["chunk"]) == (8, 5, 384) and len(known) == 40
    assert 0 < sum(known) <= 40 and set(known) <= {0, 1}
    assert "terrain" not in send(stream, {"command": "map_layers", "terrain": False})["map_layers"]
    assert len(first["products"]) == 13 and len(first["sensors"]) == 48
    assert len(first["selected"]["choices"][0]["inputs"]) == 48
    assert len(first["selected"]["person"]["goals"]) <= 5
    send(stream,{"command":"select_animal","animal":0})
    animal=snapshot(stream)["selected_animal"]
    assert animal["species"] == "Rabbit" and animal["genome_nodes"] >= 53
    assert animal["choices"] and len(animal["choices"][0]["inputs"]) == 48
    send(stream,{"command":"select","agent":0})
    send(stream, {"command":"time_of_day","hour":20.0})
    phase = snapshot(stream)
    assert phase["tick"] == first["tick"] and phase["clock"]["night"]
    assert abs(phase["clock"]["speed_multiplier"]-0.65) < 0.001
    send(stream,{"command":"mortality_test","market":0,"count":5})
    send(stream,{"command":"step","ticks":200})
    before_save=snapshot(stream)
    assert before_save["stats"]["deaths"]>=5 and len(before_save["job_boards"])==3
    # Find clear editor sites from the actual map and verify exact stock addition.
    def clear(x,y):
        from math import hypot
        return all(hypot(x-f["x"],y-f["y"])>f["radius"]+25 for f in before_save["wildlife"]["forests"]) and all(hypot(x-m["x"],y-m["y"])>500 for m in before_save["markets"]) and all(hypot(x-p["x"],y-p["y"])>50 for p in before_save["patches"]) and hypot(x-1500,y-1380)>420
    sites=[(x,y) for x in range(100,2600,150) for y in range(100,1800,150) if clear(x,y)]
    assert sites
    x,y=sites[0]
    send(stream,{"command":"add_resource","kind":7,"x":x,"y":y,"quantity":1234})
    added=snapshot(stream)
    assert added["patches"][-1]["stock"] == 1234 and added["patches"][-1]["renewal"] == 0
    assert "x" in added["selected"] and "y" in added["selected"]
    send(stream,{"command":"set_persona","market":0,"profile":3})
    assert snapshot(stream)["urban"]["villages"][0]["persona"]=="Rocky"
    send(stream, {"command": "save"})
    send(stream, {"command": "step", "ticks": 50})
    expected = snapshot(stream)
    send(stream, {"command": "load"})
    send(stream, {"command": "step", "ticks": 50})
    restored = snapshot(stream)
    assert restored["tick"] == expected["tick"]
    assert restored["stats"] == expected["stats"]
    assert restored["urban"] == expected["urban"]
    assert restored["accounts"] == expected["accounts"]
    assert restored["clock"] == expected["clock"]
    assert restored["shelters"] == expected["shelters"]
    assert restored["job_boards"] == expected["job_boards"]
    assert restored["graveyards"] == expected["graveyards"]
    assert restored["patches"] == expected["patches"]
    assert restored["wildlife"] == expected["wildlife"]
    assert restored["schools"] == expected["schools"]
    assert restored["education"] == expected["education"]
    assert restored["taverns"] == expected["taverns"]
    assert restored["farms"] == expected["farms"]
    assert restored["population_life"] == expected["population_life"]
    assert restored["selected"]["person"] == expected["selected"]["person"]
    send(stream, {"command":"advance_days", "days":1})
    for _ in range(1000):
        advanced = snapshot(stream)
        if advanced["tick"] >= restored["tick"] + 3000 and advanced["paused"]:
            break
    assert advanced["tick"] == restored["tick"] + 3000 and advanced["paused"]
    print(json.dumps({"protocol": "passed", "checkpoint_replay": "passed",
                      "population": restored["allocated_agents"], "tick": restored["tick"]}, indent=2))
