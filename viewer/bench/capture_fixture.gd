extends SceneTree
# Saves the offline town fixture at all three fidelity levels for visual review.
# Needs a rendering window: godot --path viewer --script res://bench/capture_fixture.gd -- <fixture.json> <out-dir>
class OfflineView extends "res://main.gd":
	func send(_command: Dictionary) -> void:
		pass
	func _connect() -> void:
		pass
	func _process(_delta: float) -> void:
		pass

const SHOTS := [["level1", 1.0], ["level2", 9.0], ["level3", 18.0]]

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var args := OS.get_cmdline_user_args()
	assert(args.size() >= 2, "Pass a fixture path and an output directory")
	var world: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(args[0])).world
	DirAccess.make_dir_recursive_absolute(args[1])
	root.size = Vector2i(1440, 900)
	var ui: Control = OfflineView.new()
	root.add_child(ui)
	ui.size = Vector2(1440, 900)
	await process_frame
	var metadata := JSON.stringify(world).to_utf8_buffer()
	var packet := PackedByteArray()
	packet.resize(32)
	packet.encode_u32(0, 0x31534e58)
	packet.encode_u16(4, 2)
	packet.encode_u16(6, 1)
	packet.encode_u32(24, metadata.size())
	packet.append_array(metadata)
	ui._decode(packet)
	var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	if fixture.has("map_layers"):
		var started := Time.get_ticks_usec()
		ui.field.terrain.set_map_layers(fixture.map_layers)
		print("Built terrain texture in %.1f ms" % ((Time.get_ticks_usec()-started)/1000.0))
	var market: Dictionary = world.markets[0]
	for shot in SHOTS:
		ui.field.magnification = shot[1]
		ui.field.camera_center = Vector2(market.x, market.y) if shot[1] > 1.0 else Vector2(ui.field.map_width, ui.field.map_height) * 0.5
		ui.field._resize()
		var target := SHOTS.find(shot) + 1
		while ui.field.fidelity < target and ui.field.magnification < 4000.0:
			ui.field.magnification *= 1.25
			ui.field._resize()
		for i in range(4):
			await process_frame
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(args[1].path_join(shot[0] + ".png"))
		print("Captured %s at fidelity %s" % [shot[0], ui.field.fidelity])
	quit()
