extends SceneTree
# Checks minimap camera teleports and pass-through input under real GUI dispatch.
# Usage: godot --path viewer --script res://bench/input_repro.gd -- <out_dir>
var field: Control
var frame := 0
var log_lines: Array[String] = []
var out_dir := "."

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		out_dir = args[0]
	DirAccess.make_dir_recursive_absolute(out_dir)
	call_deferred("setup")

func setup() -> void:
	field = load("res://field.gd").new()
	root.add_child(field)
	field.size = Vector2(900, 640)
	var data := PackedByteArray()
	data.resize(16)
	field.receive({"map_width": 3072.0, "map_height": 1920.0, "paused": true, "markets": [], "patches": [], "roads": [], "convoys": []}, data, 1)
	field.magnification = 3.0
	field._resize()
	var mm: Control = field.minimap
	log_lines.append("minimap rect=%s global=%s" % [Rect2(mm.position, mm.size), mm.get_global_rect()])
	var probe: Vector2 = (mm.get_global_rect().get_center() - field.offset) / field.zoom
	var data2 := PackedByteArray()
	data2.resize(16)
	data2.encode_u32(0, 4242)
	data2.encode_float(4, probe.x)
	data2.encode_float(8, probe.y)
	data2[14] = 100
	field.receive({"map_width": 3072.0, "map_height": 1920.0, "paused": true, "markets": [], "patches": [], "roads": [], "convoys": []}, data2, 1)
	log_lines.append("probe agent at world %s under minimap centre" % probe)

func _button(at: Vector2, index: int, pressed: bool) -> void:
	var e := InputEventMouseButton.new()
	e.button_index = index
	e.pressed = pressed
	e.position = at
	root.push_input(e)

func _move(at: Vector2, mask: int) -> void:
	var e := InputEventMouseMotion.new()
	e.position = at
	e.relative = Vector2(6, 4)
	e.button_mask = mask
	root.push_input(e)

func _process(_delta: float) -> bool:
	frame += 1
	var mm: Control = field.minimap
	var mm_c := mm.position + mm.size * 0.5
	match frame:
		10:
			field.camera_center = Vector2(1536, 960)
			field._resize()
			_button(Vector2(100, 100), MOUSE_BUTTON_LEFT, true)
		20, 30, 40, 50:
			_move(Vector2(100, 100) + Vector2(30, 30) * (frame / 10 - 1), MOUSE_BUTTON_MASK_LEFT)
		60:
			_move(mm.position + Vector2(30, 30), MOUSE_BUTTON_MASK_LEFT)
			log_lines.append("field-drag-over-minimap: center=%s" % field.camera_center)
		70:
			_button(mm.position + Vector2(30, 30), MOUSE_BUTTON_LEFT, false)
		80:
			_button(mm_c, MOUSE_BUTTON_LEFT, true)
		90, 100, 110:
			_move(mm_c + Vector2(20, 15) * (frame / 10 - 8), MOUSE_BUTTON_MASK_LEFT)
		120:
			_move(mm.position + mm.size + Vector2(40, 40), MOUSE_BUTTON_MASK_LEFT)
			log_lines.append("minimap-drag-off-edge: center=%s" % field.camera_center)
		130:
			_button(mm.position + mm.size + Vector2(40, 40), MOUSE_BUTTON_LEFT, false)
		140:
			# Middle-drag that STARTS on the minimap should pan the field.
			field.camera_center = Vector2(1536, 960)
			field._resize()
			_button(mm_c, MOUSE_BUTTON_MIDDLE, true)
		150, 160, 170:
			_move(mm_c + Vector2(-15, -10) * (frame / 10 - 14), MOUSE_BUTTON_MASK_MIDDLE)
		180:
			_button(mm_c, MOUSE_BUTTON_MIDDLE, false)
			log_lines.append("middle-drag-on-minimap: center=%s" % field.camera_center)
		190:
			# Wheel over the minimap should zoom the field.
			_button(mm_c, MOUSE_BUTTON_WHEEL_UP, true)
		200:
			log_lines.append("wheel-over-minimap: magnification=%s" % field.magnification)
		210:
			# Foreign release over the minimap: no select, no teleport.
			field.selected = -1
			field.camera_center = Vector2(1536, 960)
			field._resize()
			_button(Vector2(100, 100), MOUSE_BUTTON_LEFT, true)
		215:
			_button(mm_c, MOUSE_BUTTON_LEFT, false)
		220:
			log_lines.append("ghost-release: selected=%s center=%s" % [field.selected, field.camera_center])
		230:
			var file := FileAccess.open(out_dir.path_join("input.txt"), FileAccess.WRITE)
			for line in log_lines:
				file.store_line(line)
				print(line)
			file.close()
			quit()
			return true
	return false
