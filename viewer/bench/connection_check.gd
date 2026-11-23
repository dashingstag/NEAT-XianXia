extends SceneTree

func _initialize() -> void:
	call_deferred("verify_connection")

## Waits until `check` returns true on a fresh frame, or fails with `what`.
func _wait(view, check: Callable, what: String, ms := 15000) -> bool:
	var deadline := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if check.call(view):
			return true
	push_error(what)
	quit(1)
	return false

static func _inspected(view) -> int:
	var info: Variant = view.state.get("selected")
	return int(info.get("id", -1)) if info is Dictionary else -1

func verify_connection() -> void:
	var view = preload("res://main.gd").new()
	root.add_child(view)
	if not await _wait(view, func(v): return v.connected and v.counter.text.contains("BALANCED"),
			"Viewer did not decode a balanced snapshot from its assigned backend"):
		return
	print("Verified connected Godot viewer and balanced snapshot")
	# A reconnect keeps the person the player is inspecting (#88).
	view._select_agent(5)
	if not await _wait(view, func(v): return _inspected(v) == 5, "Selecting agent 5 never reached a snapshot"):
		return
	view._connect()
	view.state = {}  # only a snapshot from the new connection may satisfy the check
	if not await _wait(view, func(v): return v.connected and not v.restoring_selection and _inspected(v) == 5,
			"Reconnect lost the selected person"):
		return
	print("Verified reconnect keeps the selected person")
	# A selection the backend does not have falls back to agent 0.
	view.field.selected = 1_000_000
	view._connect()
	view.state = {}
	if not await _wait(view, func(v): return v.connected and not v.restoring_selection and _inspected(v) == 0 and v.field.selected == 0,
			"Reconnect with an unknown person did not fall back to agent 0"):
		return
	print("Verified reconnect falls back to agent 0 for an unknown person")
	quit()
