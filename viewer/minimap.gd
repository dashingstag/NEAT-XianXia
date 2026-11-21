extends Control
# Corner overview of the whole map: terrain, explored chunks, settlements and
# the camera's view. Click or drag to move the camera.
const WIDTH := 220.0
var field: Control
var dragging := false

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_PASS
	clip_contents = true
	z_index = 1

func layout() -> void:
	size = Vector2(WIDTH, WIDTH*field.map_height/field.map_width)
	position = field.size - size - Vector2(12, 12)
	queue_redraw()

func _draw() -> void:
	var k: float = size.x/float(field.map_width)
	var map := Rect2(Vector2.ZERO, size)
	var terrain: Node2D = field.terrain
	var classic: bool = field._classic_terrain()
	draw_rect(map, Color("93a575"))
	if not classic and terrain.ground.texture != null:
		draw_texture_rect(terrain.ground.texture, map, false)
	if not classic and terrain.fog_texture != null:
		draw_texture_rect(terrain.fog_texture, Rect2(terrain.fog_rect.position*k, terrain.fog_rect.size*k), false)
	for market in field.world.get("markets", []):
		draw_circle(Vector2(market.x, market.y)*k, 3.0, Color("f3d27a"))
		draw_circle(Vector2(market.x, market.y)*k, 3.0, Color("3b2a17"), false, 1.0)
	var view := Rect2((-field.offset/field.zoom)*k, field.size/field.zoom*k).intersection(map)
	draw_rect(view, Color(1, 1, 1, 0.9), false, 1.5)
	draw_rect(map, Color(0.06, 0.12, 0.09, 0.95), false, 2.0)

func _gui_input(event: InputEvent) -> void:
	# Left presses and drags that begin on the minimap move the camera.
	# Everything else falls through to the field so pan, zoom and select
	# keep working over this corner.
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if not event.pressed:
			accept_event()
			if not dragging:
				return
		dragging = event.pressed
	elif event is InputEventMouseMotion:
		# A release outside the window never reaches us; clear stale drags.
		dragging = dragging and (event.button_mask & MOUSE_BUTTON_MASK_LEFT) != 0
		if not dragging:
			return
	else:
		return
	var target: Vector2 = event.position.clamp(Vector2.ZERO, size)/size*Vector2(field.map_width, field.map_height)
	field.camera_center = target
	if field.follow_selection:
		field.follow_selection = false
		field.follow_cancelled.emit()
	field._resize()
	accept_event()
