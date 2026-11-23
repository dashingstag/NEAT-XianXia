"""Check the streamed-building contract (contracts/README.md) over the native TCP API.

The server must be started on a checkpoint with more than 256 buildings, so
that the stream needs at least two `building_batch` frames. The assembler
below follows the viewer's rules in viewer/main.gd `_decode`: batches write
into a persistent pending array at `building_offset`, and `building_count`
resizes that array and publishes it.
"""
import json
import struct

from check_protocol import frame, send, snapshot

BATCH = 256


class Assembler:
    def __init__(self):
        self.pending = []
        self.published = None
        self.offsets = []
        self.commits = []

    def apply(self, message):
        if "building_batch" in message:
            start, batch = message["building_offset"], message["building_batch"]
            assert start % BATCH == 0, f"offset {start} is not a batch boundary"
            assert 0 < len(batch) <= BATCH, f"batch at {start} has {len(batch)} rows"
            self.offsets.append(start)
            if len(self.pending) < start + len(batch):
                self.pending.extend([None] * (start + len(batch) - len(self.pending)))
            self.pending[start:start + len(batch)] = batch
            return True
        if "building_count" in message:
            count = message["building_count"]
            del self.pending[count:]
            self.pending.extend([None] * (count - len(self.pending)))
            assert None not in self.pending, "commit published rows no batch supplied"
            self.published = list(self.pending)
            self.commits.append(count)
            return True
        return False


def pump(stream, assembler, until_commits, snapshots=None, limit=400):
    """Read frames until `until_commits` new commits arrive; return the XNC1 replies seen."""
    target = len(assembler.commits) + until_commits
    replies = []
    for _ in range(limit):
        data = frame(stream)
        if data[:4] == b"XNS1":
            if snapshots is not None:
                snapshots.append(data)
            continue
        message = json.loads(data[4:])
        if not assembler.apply(message):
            replies.append(message)
        if len(assembler.commits) >= target:
            return replies
    raise AssertionError(f"Expected {until_commits} building commit(s); saw {assembler.commits}")


def command(stream, assembler, value):
    """Send a command while streaming; building frames are applied, not mistaken for the reply."""
    data = json.dumps(value).encode()
    stream.sendall(struct.pack("<I", len(data)) + data)
    for _ in range(400):
        message = frame(stream)
        if message[:4] != b"XNC1":
            continue
        message = json.loads(message[4:])
        if assembler.apply(message):
            continue
        assert message["ok"], message
        return message
    raise AssertionError(f"No reply to {value['command']}")


def streamed_meta(data):
    """Decode only the snapshot metadata (row counts are checked by check_protocol.snapshot)."""
    meta_len = struct.unpack_from("<HHQIIII", data, 4)[5]
    return json.loads(data[32:32 + meta_len])


def check(stream):
    send(stream, {"command": "pause", "value": True})
    expected = snapshot(stream)["urban"]["buildings"]
    assert len(expected) > BATCH, f"fixture has {len(expected)} buildings; need more than {BATCH}"
    send(stream, {"command": "save"})

    # 1-2. Enable streaming: snapshots stop embedding rows; batches rebuild the list.
    assembler = Assembler()
    send(stream, {"command": "stream_buildings"})
    snapshots = []
    pump(stream, assembler, 1, snapshots)
    assert sorted(assembler.offsets) == list(range(0, len(expected), BATCH)), assembler.offsets
    assert assembler.published == expected, "streamed rows differ from the embedded rows"
    command(stream, assembler, {"command": "step", "ticks": 1})
    for _ in range(3):
        data = frame(stream)
        if data[:4] != b"XNS1":
            assembler.apply(json.loads(data[4:]))
            continue
        snapshots.append(data)
    assert snapshots, "no snapshot arrived while streaming"
    for data in snapshots:
        meta = streamed_meta(data)
        assert meta.get("building_stream") is True, "snapshot does not mark building_stream"
        assert meta["urban"]["buildings"] == [], "streamed snapshot still embeds building rows"

    # 3-4. Reset clears the stream before any new row, and the commit truncates.
    before = len(assembler.offsets)
    command(stream, assembler, {"command": "reset", "agents": 1000, "seed": 77})
    pump(stream, assembler, 1)
    assert assembler.commits[-1] == 0 and len(assembler.offsets) == before, \
        "reset did not send building_count 0 before new rows"
    assert assembler.published == []

    # Load restores the saved world: cleared first, then every batch again. The
    # reset world may publish its own rows first, so wait for the loaded count.
    command(stream, assembler, {"command": "load"})
    first, rows_from = len(assembler.commits), len(assembler.offsets)
    for _ in range(10):
        pump(stream, assembler, 1)
        if assembler.commits[-1] == len(expected):
            break
    after_load = assembler.commits[first:]
    assert after_load[0] == 0, f"load did not clear the stream first: {after_load}"
    assert after_load[-1] == len(expected), after_load
    assert set(range(0, len(expected), BATCH)) <= set(assembler.offsets[rows_from:])
    assert assembler.published == expected, "rows after load differ from the saved world"

    print(json.dumps({"building_stream": "passed", "buildings": len(expected),
                      "batches": len(range(0, len(expected), BATCH)),
                      "commits": assembler.commits}, indent=2))
