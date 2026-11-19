extends SceneTree
# Standalone rendered regression fixture; does not connect to or change the live world.
var field: Control
var frame := 0
var durations: Array[float] = []
var draw_calls: Array[int] = []
var last_us := 0
var legacy := false
var output := ""
func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	legacy = "--legacy" in args
	if "--out" in args:
		output = args[args.find("--out")+1]
	Engine.max_fps = 0
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	call_deferred("setup")
func setup() -> void:
	field = load("res://bench/legacy_field.gd" if legacy else "res://field.gd").new()
	root.add_child(field)
	field.size = Vector2(900,640)
	field.selected = 9999
	push_snapshot()
	field.set_fidelity(3)
	field.camera_center = Vector2(500,320)
	field._resize()
	last_us = Time.get_ticks_usec()
func push_snapshot() -> void:
	var data := PackedByteArray()
	data.resize(10000*16)
	for i in range(10000):
		var at := i*16
		data.encode_u32(at,i)
		data.encode_float(at+4,450.0+float(i%100)+sin(float(frame)*0.1)*3.0)
		data.encode_float(at+8,270.0+float(i/100))
		data[at+12] = 1
		data[at+14] = 100
		data[at+15] = i%3
	field.receive({"map_width":1024,"map_height":640,"paused":false,"markets":[],"patches":[],"roads":[],"convoys":[]},data,10000)
func _process(_delta: float) -> bool:
	if field == null:
		return false
	var now := Time.get_ticks_usec()
	if frame >= 30:
		durations.append(float(now-last_us)/1000.0)
		draw_calls.append(RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME))
	last_us = now
	frame += 1
	if frame%6 == 0:
		push_snapshot()
	if frame in [60,90,120]:
		field.magnification = 6.0 if frame == 90 else 8.0
		field._resize()
	if frame >= 150:
		assert(field.draw_indices.size()==2500)
		if not legacy:
			assert(field.draw_indices.has(9999))
			assert(field.detail_mesh.multimesh.instance_count==2500)
			assert(field.detail_mesh.multimesh.buffer.size()==2500*12)
		var sum_ms := 0.0
		for duration in durations:
			sum_ms += duration
		durations.sort()
		draw_calls.sort()
		var result := {"legacy":legacy,"sampled_people":10000,"detailed_people":2500,"frames":durations.size(),"mean_frame_ms":sum_ms/durations.size(),"p95_frame_ms":durations[int(durations.size()*0.95)],"median_draw_calls":draw_calls[draw_calls.size()/2],"display_driver":DisplayServer.get_name(),"renderer":RenderingServer.get_current_rendering_method()}
		print(JSON.stringify(result))
		if output != "":
			var file := FileAccess.open(output,FileAccess.WRITE)
			file.store_string(JSON.stringify(result,"  "))
		quit()
	return false
