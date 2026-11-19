extends SceneTree

func _initialize() -> void:
	call_deferred("verify_connection")

func verify_connection() -> void:
	var view = preload("res://main.gd").new()
	root.add_child(view)
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if view.connected and view.counter.text.contains("BALANCED"):
			print("Verified connected Godot viewer and balanced snapshot")
			quit()
			return
	push_error("Viewer did not decode a balanced snapshot from its assigned backend")
	quit(1)
