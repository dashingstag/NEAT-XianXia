extends SceneTree
# Minimap flicker reproduction: runs the real field.gd off a synthetic X4 world,
# pushes snapshots at ~10Hz, refreshes the explored mask like the live viewer,
# and diffs the minimap region between consecutive frames.
# Usage: godot --path viewer --script res://bench/flicker_repro.gd -- <out_dir> [--classic]
var field: Control
var frame := 0
var prev_crop: Image
var out_dir := "."
var classic := false
var diffs: Array = []
var saved := 0

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		out_dir = args[0]
	classic = "--classic" in args
	DirAccess.make_dir_recursive_absolute(out_dir)
	Engine.max_fps = 0
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	call_deferred("setup")

func setup() -> void:
	field = load("res://field.gd").new()
	root.add_child(field)
	field.size = Vector2(900, 640)
	push_snapshot()
	if not classic:
		field.terrain.set_map_layers(layers(true))
	capture_loop()

func snap_world() -> Dictionary:
	return {
		"seed": 42,
		"map": {"size": "Classic" if classic else "X4", "seed": 42, "version": 3, "home_x": 3072.0, "home_y": 1920.0},
		"map_width": 3072.0 if classic else 6144.0,
		"map_height": 1920.0 if classic else 3840.0,
		"paused": false,
		"clock": {"sunlight": 1.0, "night": false, "hour": 12.0},
		"markets": [
			{"id": 0, "x": 1500.0, "y": 1000.0, "population": 300, "stage": "Town", "name": "A", "housing_blocks": 40},
			{"id": 1, "x": 3500.0, "y": 2200.0, "population": 120, "stage": "Village", "name": "B", "housing_blocks": 10},
		],
		"patches": [], "roads": [], "convoys": [], "banks": [], "schools": [], "taverns": [],
		"farms": [], "graveyards": [], "shelters": [], "job_boards": [], "fertility": [],
		"urban": {"buildings": [], "zones": [], "cleared": []},
		"wildlife": {"forests": [], "animals": [], "grass": []},
	}

func push_snapshot() -> void:
	var count := 400
	var data := PackedByteArray()
	data.resize(count * 16)
	var w := 3072.0 if classic else 6144.0
	var h := 1920.0 if classic else 3840.0
	for i in range(count):
		var at := i * 16
		data.encode_u32(at, i)
		data.encode_float(at + 4, fmod(float(i) * 37.0 + float(frame) * 0.6, w))
		data.encode_float(at + 8, fmod(float(i) * 53.0, h))
		data[at + 12] = 1
		data[at + 13] = i % 6
		data[at + 14] = 100
		data[at + 15] = i % 3
	field.receive(snap_world(), data, count)

func b64raw(n: int, fn: Callable) -> String:
	var bytes := PackedByteArray()
	bytes.resize(n)
	for i in range(n):
		bytes[i] = int(fn.call(i)) & 255
	return Marshalls.raw_to_base64(bytes)

func layers(with_terrain: bool) -> Dictionary:
	var out := {"map": {"size": "X4", "seed": 42, "version": 3, "home_x": 3072.0, "home_y": 1920.0}}
	var known_n := 160
	out["explored"] = {"cols": 16, "rows": 10, "chunk": 384,
		"known": b64raw(known_n, func(i): return 1 if (i + frame / 30) % 3 else 0)}
	if with_terrain:
		var n := 96 * 60
		out["terrain"] = {"cols": 96, "rows": 60, "cell": 64.0,
			"classes": b64raw(n, func(i): return [0, 2, 5, 5, 5, 1][(i * 7) % 6]),
			"elevation": b64raw(n, func(i): return (i * 13) % 255),
			"qi": b64raw(n, func(i): return (i * 29) % 255),
			"regions": b64raw(n, func(i): return 255),
			"region_labels": [], "rivers": []}
	return out

func _process(_delta: float) -> bool:
	if field == null:
		return false
	frame += 1
	if frame % 6 == 0:
		push_snapshot()
	# Swap the fog texture mid-cycle, 3 frames after a snapshot, so the stale-RID
	# window overlaps frames where neither field nor minimap is queued.
	if not classic and frame % 90 == 33:
		field.terrain.set_map_layers(layers(false))
	if frame % 45 == 0:
		field.camera_center += Vector2(300, 150)
		field._resize()
	if frame == 200:
		field.magnification = 4.0
		field._resize()
	if frame >= 420:
		finish()
		return true
	return false

func capture_loop() -> void:
	while true:
		await RenderingServer.frame_post_draw
		if field == null:
			continue
		var img: Image = root.get_texture().get_image()
		var rect := Rect2i(Vector2i(field.get_global_rect().position + field.minimap.position), Vector2i(field.minimap.size))
		var crop := img.get_region(rect)
		crop.convert(Image.FORMAT_RGB8)
		var white_px := 0
		for y in range(0, crop.get_height(), 2):
			for x in range(0, crop.get_width(), 2):
				var p := crop.get_pixel(x, y)
				if p.r8 + p.g8 + p.b8 > 730:
					white_px += 1
		if prev_crop != null and prev_crop.get_size() == crop.get_size():
			var changed := 0
			var maxd := 0
			for y in range(0, crop.get_height(), 2):
				for x in range(0, crop.get_width(), 2):
					var a := crop.get_pixel(x, y)
					var b := prev_crop.get_pixel(x, y)
					var d: int = absi(a.r8 - b.r8) + absi(a.g8 - b.g8) + absi(a.b8 - b.b8)
					if d > 40:
						changed += 1
						if d > maxd:
							maxd = d
			diffs.append({"frame": frame, "changed_px": changed, "white_px": white_px})
			if changed > 400 and saved < 12:
				crop.save_png(out_dir.path_join("f%04d_now.png" % frame))
				prev_crop.save_png(out_dir.path_join("f%04d_prev.png" % frame))
				saved += 1
				print("frame %d: %d changed px (dmax %d)" % [frame, changed, maxd])
		prev_crop = crop

func finish() -> void:
	var hot := diffs.filter(func(d): return d.changed_px > 400).size()
	var mild := diffs.filter(func(d): return d.changed_px > 10).size()
	var whites := diffs.filter(func(d): return d.get("white_px", 0) > 1000).map(func(d): return d.frame)
	print("RESULT frames=%d diff>10px=%d diff>400px=%d white_frames=%s" % [diffs.size(), mild, hot, str(whites)])
	var file := FileAccess.open(out_dir.path_join("diffs.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(diffs))
	file.close()
