extends Control
signal agent_selected(id: int)

var world: Dictionary = {}
var points := PackedVector2Array()
var ids := PackedInt32Array()
var meshes: Array[MultiMeshInstance2D] = []
var selected := 0
var zoom := 1.0
var offset := Vector2.ZERO
var magnification := 1.0
var camera_center := Vector2(512,320)
var fidelity := 1
var actions := PackedInt32Array()
var kinds := PackedInt32Array()
var healths := PackedInt32Array()
var draw_points := PackedVector2Array()
var moving := PackedByteArray()
var phases := PackedFloat32Array()
var draw_indices := PackedInt32Array()
var animation_time := 0.0
var panning := false
var palette := [Color("e5c985"), Color("90c99a"), Color("e2bb78"), Color("99c8d7"), Color("cbb1de"), Color("df9c78")]

func _ready() -> void:
	custom_minimum_size = Vector2(650, 480)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	clip_contents = true
	for region in range(3):
		var instance := MultiMeshInstance2D.new()
		instance.multimesh = MultiMesh.new()
		instance.multimesh.transform_format = MultiMesh.TRANSFORM_2D
		instance.multimesh.use_colors = true
		var quad := QuadMesh.new()
		quad.size = Vector2(3.0, 4.0)
		instance.multimesh.mesh = quad
		add_child(instance)
		meshes.append(instance)
	resized.connect(_resize)
	_resize()

func _resize() -> void:
	zoom = minf(size.x / 1024.0, size.y / 640.0) * magnification
	offset = size * 0.5 - camera_center * zoom
	fidelity = 1 if magnification < 2.0 else (2 if magnification < 5.0 else 3)
	for instance in meshes:
		instance.position = offset
		instance.scale = Vector2(zoom,zoom)
	_update_detail_indices()
	queue_redraw()

func receive(meta: Dictionary, records: PackedByteArray, count: int) -> void:
	world = meta
	var old_points := points.duplicate()
	var old_ids := ids.duplicate()
	points.resize(count)
	ids.resize(count)
	actions.resize(count)
	kinds.resize(count)
	healths.resize(count)
	moving.resize(count)
	phases.resize(count)
	draw_points.resize(count)
	var sizes := [0,0,0]
	for i in range(count):
		var region: int = records[i*16+15] % 3
		if region < 3:
			sizes[region] += 1
	var buffers: Array[PackedFloat32Array] = []
	for region in range(3):
		var buffer := PackedFloat32Array()
		buffer.resize(sizes[region] * 12)
		buffers.append(buffer)
	var cursors := [0,0,0]
	for i in range(count):
		var at := i * 16
		var id: int = records.decode_u32(at)
		var x: float = records.decode_float(at+4)
		var y: float = records.decode_float(at+8)
		var grouped: bool = (records[at+12] & 128) != 0
		var action: int = records[at+12] & 63
		var health: int = records[at+14]
		var region: int = records[at+15] % 3
		points[i] = Vector2(x,y)
		if i >= old_ids.size() or old_ids[i] != id:
			draw_points[i] = points[i]
			phases[i] = float(id % 31) * 0.2
		moving[i] = 1 if i < old_points.size() and old_points[i].distance_to(points[i]) > 0.1 else 0
		ids[i] = id
		actions[i] = action
		kinds[i] = records[at+13]
		healths[i] = health
		if region >= 3:
			continue
		var color: Color = palette[mini(action,palette.size()-1)] if health > 0 else Color("686a60")
		var cursor: int = cursors[region] * 12
		buffers[region][cursor] = 0.0 if grouped else 1.0
		buffers[region][cursor+3] = x
		buffers[region][cursor+5] = 0.0 if grouped else 1.0
		buffers[region][cursor+7] = y
		buffers[region][cursor+8] = color.r
		buffers[region][cursor+9] = color.g
		buffers[region][cursor+10] = color.b
		buffers[region][cursor+11] = 1.0
		cursors[region] += 1
	for region in range(3):
		meshes[region].multimesh.instance_count = sizes[region]
		meshes[region].multimesh.buffer = buffers[region]
	_update_detail_indices()
	queue_redraw()

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_MIDDLE:
			panning = event.pressed
		if event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP,MOUSE_BUTTON_WHEEL_DOWN]:
			var anchor: Vector2 = (event.position-offset)/zoom
			magnification = clampf(magnification * (1.25 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 0.8),1.0,12.0)
			_resize()
			camera_center = anchor - (event.position-size*0.5)/zoom
			_clamp_camera()
			_resize()
			accept_event()
			return
	if event is InputEventMouseMotion and panning:
		camera_center -= event.relative/zoom
		_clamp_camera()
		_resize()
		return
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var p: Vector2 = (event.position - offset) / zoom
		for convoy in world.get("convoys",[]) if fidelity == 1 else []:
			if p.distance_to(Vector2(convoy.x,convoy.y)) < 12.0:
				selected = int(convoy.leader)
				agent_selected.emit(selected)
				queue_redraw()
				return
		var best := -1
		var limit := 20.0 / maxf(1.0,magnification * 0.5)
		for i in range(points.size()):
			var d: float = p.distance_to(points[i])
			if d < limit:
				limit = d
				best = i
		if best >= 0:
			selected = ids[best]
			agent_selected.emit(selected)
			queue_redraw()

func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO,size),Color("15231d"))
	draw_set_transform(offset,0,Vector2(zoom,zoom))
	draw_rect(Rect2(0,0,1024,640),Color("8b9f6b"))
	draw_rect(Rect2(900,0,124,640),Color("4b8289"))
	draw_rect(Rect2(887,0,13,640),Color("c7bf89"))
	draw_circle(Vector2(185,205),172,Color("4c714c"))
	draw_circle(Vector2(500,460),120,Color("a1a486"))
	draw_circle(Vector2(510,454),86,Color("afb195"))
	for y in range(15,640,37):
		draw_line(Vector2(925,y),Vector2(985,y-5),Color(0.7,0.9,0.9,0.18),1)
	var markets: Array = world.get("markets",[])
	for road in world.get("roads",[]):
		var a := int(road.get("a",0))
		var b := int(road.get("b",0))
		if a >= markets.size() or b >= markets.size():
			continue
		var pa := Vector2(markets[a].x,markets[a].y)
		var pb := Vector2(markets[b].x,markets[b].y)
		draw_line(pa,pb,Color("d5bd8e") if road.built else Color(0.9,0.85,0.65,0.2),7 if road.built else 2)
	for patch in world.get("patches",[]):
		var p := Vector2(patch.x,patch.y)
		var kind := int(patch.kind)
		var fraction: float = float(patch.stock) / maxf(1.0,float(patch.capacity))
		var colors := [Color("a95d76"),Color("365a38"),Color("d0ccb1"),Color("bce1df"),Color("edc65d"),Color("c8d6df"),Color("ba7650")]
		var radius := (5.0 if kind != 1 else 9.0) * (0.45 + fraction * 0.55)
		draw_circle(p+Vector2(2,3),radius,Color(0.1,0.2,0.1,0.18))
		draw_circle(p,radius,colors[kind])
		if kind >= 4:
			draw_rect(Rect2(p-Vector2(10,7),Vector2(20,14)),Color("535448"))
			draw_arc(p,9,PI,TAU,12,colors[kind],3)
			if fidelity == 1:
				draw_string(ThemeDB.fallback_font,p+Vector2(-30,22),["Gold mine","Silver mine","Copper mine"][kind-4],HORIZONTAL_ALIGNMENT_LEFT,-1,11,colors[kind])
		if kind == 1 and fraction > 0.2:
			draw_circle(p-Vector2(3,4),radius*0.7,Color("5d8750"))
	for m in markets:
		var p := Vector2(m.x,m.y)
		draw_circle(p,42,Color(0.95,0.87,0.62,0.24))
		draw_rect(Rect2(p-Vector2(16,11),Vector2(32,22)),Color("d6c698"))
		draw_rect(Rect2(p-Vector2(19,15),Vector2(38,13)),Color("a27254"))
		var blocks := mini(int(m.housing_blocks),12)
		for j in range(blocks):
			var at := p + Vector2(cos(j*TAU/12.0),sin(j*TAU/12.0))*57.0
			draw_rect(Rect2(at,Vector2(12,9)),Color("d1bf91"))
			draw_rect(Rect2(at-Vector2(1,2),Vector2(14,5)),Color("a07153"))
		if fidelity == 1:
			draw_string(ThemeDB.fallback_font,p-Vector2(65,85),str(m.name),HORIZONTAL_ALIGNMENT_LEFT,-1,16,Color("f7eed0"))
			draw_string(ThemeDB.fallback_font,p-Vector2(65,65),str(int(m.population))+" people / "+str(int(m.get("stable_residents",0)))+" residents | "+str(m.stage),HORIZONTAL_ALIGNMENT_LEFT,-1,11,Color("eef0cc"))
	for convoy in world.get("convoys",[]) if fidelity == 1 else []:
		var p := Vector2(convoy.x,convoy.y)
		var color := Color("f4d88d") if convoy.departed else Color("e4e4be")
		draw_rect(Rect2(p-Vector2(6,4),Vector2(12,8)),color,convoy.departed,-1.0 if convoy.departed else 1.5)
		draw_circle(p+Vector2(-4,5),2,Color("443c30"))
		draw_circle(p+Vector2(4,5),2,Color("443c30"))
		draw_string(ThemeDB.fallback_font,p+Vector2(8,-4),str(int(convoy.members)),HORIZONTAL_ALIGNMENT_LEFT,-1,10,color)
	if fidelity > 1:
		for i in draw_indices:
			_draw_person(i)
	for i in range(ids.size()):
		if ids[i] == selected:
			draw_arc(points[i],11,0,TAU,24,Color("ffedaa"),2)
			break
	if fidelity == 1:
		draw_string(ThemeDB.fallback_font,Vector2(25,31),"WILLOW FOREST",HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("deebc7"))
		draw_string(ThemeDB.fallback_font,Vector2(911,31),"COAST",HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("e2f2e6"))
	draw_set_transform(Vector2.ZERO)
	draw_style_box(_hud_style(),Rect2(12,12,360,49))
	draw_string(ThemeDB.fallback_font,Vector2(23,32),"LEVEL %s / %s / %.1fx" % [fidelity,["OVERVIEW","INDIVIDUAL FEATURES","ANIMATED INDIVIDUALS"][fidelity-1],magnification],HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("f7eed0"))
	draw_string(ThemeDB.fallback_font,Vector2(23,49),"Wheel: zoom   Middle drag: pan   Detailed people: %s" % draw_indices.size(),HORIZONTAL_ALIGNMENT_LEFT,-1,11,Color("b7c7ad"))



func _hud_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.06,0.12,0.09,0.88)
	style.set_corner_radius_all(5)
	return style

func _clamp_camera() -> void:
	if magnification <= 1.01:
		camera_center = Vector2(512,320)
	else:
		camera_center.x = clampf(camera_center.x,0,1024)
		camera_center.y = clampf(camera_center.y,0,640)

func set_fidelity(level: int) -> void:
	magnification = [1.0,3.0,8.0][clampi(level-1,0,2)]
	if level > 1:
		for i in range(ids.size()):
			if ids[i] == selected:
				camera_center = points[i]
				break
	_clamp_camera()
	_resize()

func _update_detail_indices() -> void:
	draw_indices.clear()
	for instance in meshes:
		instance.visible = fidelity == 1
	if fidelity == 1:
		return
	var rect := Rect2(-offset/zoom,size/zoom).grow(12.0)
	for i in range(points.size()):
		if rect.has_point(points[i]):
			draw_indices.append(i)
			if draw_indices.size() >= 2500:
				break

func _process(delta: float) -> void:
	if fidelity == 1:
		return
	for i in draw_indices:
		draw_points[i] = draw_points[i].lerp(points[i],minf(delta*18.0,1.0))
		if not world.get("paused",true) and moving[i] > 0 and healths[i] > 0:
			phases[i] += delta * 10.0
	animation_time += delta if not world.get("paused",true) else 0.0
	queue_redraw()

func _draw_person(i: int) -> void:
	var p := draw_points[i] if fidelity == 3 else points[i]
	var active: bool = not world.get("paused",true) and healths[i] > 0
	var walk: float = sin(phases[i]) * 1.1 if fidelity == 3 and active and moving[i] > 0 else 0.0
	var work: float = sin(animation_time*6.0+float(ids[i]%11))*0.7 if fidelity == 3 and active and moving[i] == 0 and actions[i] in [1,4,6,8] else 0.0
	var tunic := Color.from_hsv(float((ids[i]*47)%360)/360.0,0.40,0.62)
	var skin: Color = [Color("cf9c74"),Color("dcb58c"),Color("a97855"),Color("edc5a0")][ids[i]%4]
	if healths[i] == 0:
		draw_line(p-Vector2(2,0),p+Vector2(2,0),Color("77776d"),2)
		return
	draw_ellipse_shadow(p)
	draw_line(p+Vector2(-0.8,1.2),p+Vector2(-0.8,3.2+walk),Color("3d3832"),0.9)
	draw_line(p+Vector2(0.8,1.2),p+Vector2(0.8,3.2-walk),Color("3d3832"),0.9)
	draw_rect(Rect2(p-Vector2(1.6,1.4),Vector2(3.2,3.5)),tunic)
	draw_line(p+Vector2(-1.7,-0.2),p+Vector2(-2.1,1.7-walk*0.6),skin,0.8)
	draw_line(p+Vector2(1.7,-0.2),p+Vector2(2.1+work,1.7+walk*0.6),skin,0.8)
	draw_circle(p-Vector2(0,2.0),1.25,skin)
	draw_arc(p-Vector2(0,2.3),1.1,PI,TAU,8,Color("352d29"),0.8)
	draw_circle(p-Vector2(0.5,2.2),0.9,Color("45352c"))
	draw_line(p+Vector2(-1.5,0.8),p+Vector2(1.5,0.8),Color("524538"),0.55)
	draw_rect(Rect2(p+Vector2(-1.1,-0.7),Vector2(1.8,2.0)),Color("85704e"))
	draw_circle(p+Vector2(1.5,1.1),0.55,Color("d8b456"))
	if actions[i] in [1,4,8]:
		draw_line(p+Vector2(2.2,0.5),p+Vector2(3.5+work,-2.2-work),Color("7a5535"),0.55)
		draw_line(p+Vector2(2.5+work,-2.2-work),p+Vector2(4.5+work,-2.2-work),Color("c0c4b6"),0.7)
	if i < ids.size() and ids[i] == selected:
		draw_arc(p,4.5,0,TAU,20,Color("fff1a6"),0.35)
		if fidelity == 3:
			draw_string(ThemeDB.fallback_font,p+Vector2(-4,-5),"#%s" % ids[i],HORIZONTAL_ALIGNMENT_LEFT,-1,2,Color("fff1cf"))

func draw_ellipse_shadow(p: Vector2) -> void:
	draw_circle(p+Vector2(0.5,2.2),2.3,Color(0.08,0.13,0.08,0.22))
