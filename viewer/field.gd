extends Control
signal agent_selected(id: int)
signal animal_selected(id: int)
signal site_selected(site: Dictionary)
var show_zones: bool = false
var show_trade: bool = false
var show_resources: bool = false
var settlement_label_boxes: Dictionary = {}
var hovered_settlement_label := -1
signal resource_placed(kind: int, position: Vector2)
signal follow_cancelled
signal board_selected(market: int)

var world: Dictionary = {}
var points := PackedVector2Array()
var ids := PackedInt32Array()
var meshes: Array[MultiMeshInstance2D] = []
var selected := 0
var selected_animal := -1
var pick_mode := "Auto"
var placement_kind := -1
var follow_selection := false
var focus_target := Vector2.ZERO
var has_focus_target := false
var site_focus := false
var tree_layouts: Dictionary = {}
var zoom := 1.0
var offset := Vector2.ZERO
var magnification := 1.0
var camera_center := Vector2(1536,960)
var map_width := 3072.0
var map_height := 1920.0
var fidelity := 1
var actions := PackedInt32Array()
var kinds := PackedInt32Array()
var healths := PackedInt32Array()
var rested_flags := PackedByteArray()
var sleeping_flags := PackedByteArray()
var hidden_flags := PackedByteArray()
var carrying_flags := PackedByteArray()
var child_flags := PackedByteArray()
var draw_points := PackedVector2Array()
var moving := PackedByteArray()
var phases := PackedFloat32Array()
var draw_indices := PackedInt32Array()
var animation_time := 0.0
var panning := false
var pan_button := 0
var pan_start := Vector2.ZERO
const DRAG_THRESHOLD := 5.0
var detail_mesh: MultiMeshInstance2D
var terrain: Node2D
var detail_material: ShaderMaterial
var atlas_view: SubViewport
var spatial: Dictionary = {}
var camera_dirty := false
var overview_dirty := true
var record_cache := PackedByteArray()
var snapshot_age := 0.0
var snapshot_interval := 0.1
var snapshot_arrival := 0.0
var shader_clock := 0.0
var hud_style: StyleBoxFlat
var selected_index := -1
var detail_buffer := PackedFloat32Array()
const CELL := 64.0
var palette := [Color("e5c985"), Color("90c99a"), Color("e2bb78"), Color("99c8d7"), Color("cbb1de"), Color("df9c78")]

# Committed Xianxia textures (assets/parked). Loaded lazily; every call site
# falls back to the procedural drawing when a file is missing.
const TEX_DIR := "res://assets/parked/"
const TREE_GLYPHS := ["tree-generic","pine","bamboo","willow","plum"]
const PATCH_GLYPHS := ["berries","timber","stone","fish"]
const MINE_GLYPHS := ["mine-gold","mine-iron","mine-iron","mine-iron"]
var _tex_cache: Dictionary = {}

func _tex(rel_path: String) -> Texture2D:
	if not _tex_cache.has(rel_path):
		_tex_cache[rel_path] = preload("res://art_texture.gd").read(TEX_DIR + rel_path)
	return _tex_cache[rel_path]

func _glyph(rel_path: String, at: Vector2, w: float, h: float, alpha := 1.0) -> bool:
	var t := _tex(rel_path)
	if t == null:
		return false
	draw_texture_rect(t, Rect2(at - Vector2(w,h)*0.5, Vector2(w,h)), false,
		Color(1,1,1,alpha))
	return true

func _ready() -> void:
	custom_minimum_size = Vector2(650, 480)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	clip_contents = true
	terrain = preload("res://terrain_layer.gd").new()
	terrain.field = self
	add_child(terrain)
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
	_build_detail_renderer()
	hud_style = StyleBoxFlat.new()
	hud_style.bg_color = Color(0.06,0.12,0.09,0.88)
	hud_style.set_corner_radius_all(5)
	resized.connect(_resize)
	mouse_exited.connect(func(): _update_label_hover(Vector2(-1, -1)))
	_resize()

func _resize() -> void:
	zoom = minf(size.x / map_width, size.y / map_height) * magnification
	offset = size * 0.5 - camera_center * zoom
	var effective_zoom := magnification/(map_width/1024.0)
	fidelity = 1 if effective_zoom < 2.0 else (2 if effective_zoom < 5.0 else 3)
	for instance in meshes:
		instance.position = offset
		instance.scale = Vector2(zoom,zoom)
	camera_dirty = true
	queue_redraw()

func receive(meta: Dictionary, records: PackedByteArray, count: int) -> void:
	world = meta
	var next_width: float = float(meta.get("map_width",3072.0))
	var next_height: float = float(meta.get("map_height",1920.0))
	if next_width != map_width or next_height != map_height:
		map_width = next_width
		map_height = next_height
		camera_center = Vector2(map_width,map_height)*0.5
		_resize()
	var light: float = float(world.get("clock",{}).get("sunlight",1.0))
	modulate = Color(0.40+light*0.60,0.48+light*0.52,0.68+light*0.32,1.0)
	detail_material.set_shader_parameter("daylight_tint",modulate)
	record_cache = records
	var now := Time.get_ticks_usec() / 1000000.0
	if snapshot_arrival > 0:
		snapshot_interval = clampf(now-snapshot_arrival,0.05,0.25)
	snapshot_arrival = now
	snapshot_age = 0.0
	var old_count := ids.size()
	points.resize(count)
	ids.resize(count)
	actions.resize(count)
	kinds.resize(count)
	healths.resize(count)
	rested_flags.resize(count)
	sleeping_flags.resize(count)
	hidden_flags.resize(count)
	carrying_flags.resize(count)
	child_flags.resize(count)
	moving.resize(count)
	phases.resize(count)
	draw_points.resize(count)
	spatial.clear()
	selected_index = -1
	for i in range(count):
		var at := i * 16
		var id: int = records.decode_u32(at)
		var p := Vector2(records.decode_float(at+4),records.decode_float(at+8))
		var same: bool = i < old_count and ids[i] == id
		draw_points[i] = points[i] if same else p
		moving[i] = 1 if same and points[i].distance_squared_to(p) > 0.01 else 0
		points[i] = p
		ids[i] = id
		actions[i] = records[at+12] & 63
		kinds[i] = records[at+13] & 15
		hidden_flags[i] = 1 if (records[at+13] & 64) != 0 else 0
		carrying_flags[i] = 1 if (records[at+13] & 32) != 0 else 0
		rested_flags[i] = 1 if (records[at+12] & 64) != 0 else 0
		sleeping_flags[i] = 1 if (records[at+13] & 128) != 0 else 0
		healths[i] = records[at+14] & 127
		child_flags[i] = 1 if (records[at+14] & 128) != 0 else 0
		if id == selected:
			selected_index = i
		var key := Vector2i(floori(p.x/CELL),floori(p.y/CELL))
		if not spatial.has(key):
			spatial[key] = []
		spatial[key].append(i)
	overview_dirty = true
	camera_dirty = true
	queue_redraw()

func _refresh_overview() -> void:
	var count := ids.size()
	var buffer := PackedFloat32Array()
	buffer.resize(count*12)
	for i in range(count):
		var at := i*12
		var grouped: bool = (record_cache[i*16+12] & 128) != 0
		var color: Color = palette[mini(actions[i],palette.size()-1)] if healths[i] > 0 else Color("686a60")
		if sleeping_flags[i] > 0:
			color = Color("b5cce3")
		buffer[at] = 0.0 if grouped or hidden_flags[i] > 0 else 1.0
		buffer[at+3] = points[i].x
		buffer[at+5] = 0.0 if grouped or hidden_flags[i] > 0 else 1.0
		buffer[at+7] = points[i].y
		buffer[at+8] = color.r
		buffer[at+9] = color.g
		buffer[at+10] = color.b
		buffer[at+11] = 1.0
	var mm := meshes[0].multimesh
	if mm.instance_count != count:
		mm.instance_count = count
	mm.buffer = buffer
	overview_dirty = false

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_update_label_hover(event.position)
	if event is InputEventMouseButton:
		if event.button_index in [MOUSE_BUTTON_LEFT,MOUSE_BUTTON_MIDDLE,MOUSE_BUTTON_RIGHT]:
			if event.pressed:
				pan_button = event.button_index
				pan_start = event.position
				panning = pan_button != MOUSE_BUTTON_LEFT
				mouse_default_cursor_shape = Control.CURSOR_DRAG if panning else Control.CURSOR_ARROW
			elif event.button_index == pan_button:
				var select_click := pan_button == MOUSE_BUTTON_LEFT and not panning
				pan_button = 0
				panning = false
				mouse_default_cursor_shape = Control.CURSOR_ARROW
				if select_click:
					_select_at(event.position)
			accept_event()
			return
		if event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP,MOUSE_BUTTON_WHEEL_DOWN]:
			var anchor: Vector2 = (event.position-offset)/zoom
			magnification = clampf(magnification * (1.25 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 0.8),1.0,12.0*(map_width/1024.0))
			zoom = minf(size.x/map_width,size.y/map_height)*magnification
			camera_center = anchor - (event.position-size*0.5)/zoom
			_clamp_camera()
			_resize()
			accept_event()
			return
	if event is InputEventMouseMotion and pan_button != 0:
		if not (event.button_mask & (1 << (pan_button-1))):
			pan_button = 0
			panning = false
			mouse_default_cursor_shape = Control.CURSOR_ARROW
			return
		var delta: Vector2 = event.relative
		if not panning:
			if event.position.distance_to(pan_start) < DRAG_THRESHOLD:
				return
			panning = true
			delta = event.position-pan_start
			mouse_default_cursor_shape = Control.CURSOR_DRAG
		if follow_selection:
			follow_selection = false
			follow_cancelled.emit()
		camera_center -= delta/zoom
		_clamp_camera()
		_resize()
		accept_event()

func _select_at(position: Vector2) -> void:
	var p: Vector2 = (position - offset) / zoom
	if placement_kind >= 0:
		var kind := placement_kind
		placement_kind = -1
		resource_placed.emit(kind,p)
		return
	var animal_best := -1
	var animal_distance := 12.0 / maxf(1.0,magnification*0.3)
	for animal in world.get("wildlife",{}).get("animals",[]) if pick_mode in ["Auto","Wildlife"] else []:
		var distance := p.distance_to(Vector2(animal.x,animal.y))
		if distance < animal_distance:
			animal_distance = distance
			animal_best = int(animal.id)
	if animal_best >= 0:
		selected_animal = animal_best
		animal_selected.emit(animal_best)
		queue_redraw()
		return

	if pick_mode in ["Auto","Buildings","Resources","Forests"]:
		var site := _pick_site(p)
		if not site.is_empty():
			site_selected.emit(site)
			return
	if pick_mode not in ["Auto","People"]:
		return

	for convoy in world.get("convoys",[]) if fidelity == 1 else []:
		if p.distance_to(Vector2(convoy.x,convoy.y)) < 12.0:
			selected = int(convoy.leader)
			agent_selected.emit(selected)
			queue_redraw()
			return
	var best := -1
	var limit := 20.0 / maxf(1.0,magnification * 0.5)
	for i in range(points.size()):
		if hidden_flags[i] > 0:
			continue
		var d: float = p.distance_to(points[i])
		if d < limit:
			limit = d
			best = i
	if best >= 0:
		selected = ids[best]
		agent_selected.emit(selected)
		queue_redraw()

func _pick_site(p: Vector2) -> Dictionary:
	if pick_mode in ["Auto","Buildings"]:
		for building in world.get("urban",{}).get("buildings",[]):
			if p.distance_to(Vector2(building.x,building.y))<18.0:
				return {"kind":"urban_buildings","id":int(building.id),"market":int(building.market)}
		for key in ["banks","schools","taverns","farms","graveyards","job_boards"]:
			var sites: Array = world.get(key,[])
			for id in range(sites.size()):
				var item: Dictionary = sites[id]
				if p.distance_to(Vector2(item.x,item.y)) < (25.0 if key=="graveyards" else 19.0):
					return {"kind":key,"id":id,"market":int(item.market)}
		for town in world.get("markets",[]):
			var centre := Vector2(town.x,town.y)
			if p.distance_to(centre)<22.0:
				return {"kind":"markets","id":int(town.id),"market":int(town.id)}
			for house in range(int(town.housing_blocks)):
				var at := _house_position(centre, house)
				if Rect2(at-Vector2(2,3),Vector2(24,20)).has_point(p):
					return {"kind":"house","id":house,"market":int(town.id)}
		for shelter in world.get("shelters",[]):
			if int(shelter.get("capacity",0))>0 and shelter.get("camp",false) and p.distance_to(Vector2(shelter.x,shelter.y))<15.0:
				return {"kind":"shelters","id":int(shelter.id),"market":-1}
	if pick_mode in ["Auto","Resources"]:
		for patch in world.get("patches",[]):
			if int(patch.kind) in [4,5,6,7] and int(patch.stock)==0:
				continue
			if p.distance_to(Vector2(patch.x,patch.y))<17.0:
				return {"kind":"patches","id":int(patch.id),"market":-1}
	if pick_mode=="Forests":
		var forests: Array = world.get("wildlife",{}).get("forests",[])
		for id in range(forests.size()):
			var forest: Dictionary = forests[id]
			if p.distance_to(Vector2(forest.x,forest.y))<float(forest.radius):
				return {"kind":"forests","id":id,"market":-1}
	if pick_mode=="Buildings":
		var roads: Array = world.get("roads",[])
		for id in range(roads.size()):
			var road: Dictionary = roads[id]
			var towns: Array = world.get("markets",[])
			if not road.built or int(road.a)>=towns.size() or int(road.b)>=towns.size():
				continue
			var a := Vector2(towns[int(road.a)].x,towns[int(road.a)].y)
			var b := Vector2(towns[int(road.b)].x,towns[int(road.b)].y)
			if p.distance_to(Geometry2D.get_closest_point_to_segment(p,a,b))<8.0:
				return {"kind":"roads","id":id,"market":-1}
	return {}

func _tree_points(id: int, radius: float) -> PackedVector2Array:
	var key := "%s/%s/%s" % [int(world.get("seed",0)),id,radius]
	if not tree_layouts.has(key):
		var random := RandomNumberGenerator.new()
		random.seed = int(world.get("seed",0))+id*7919+104729
		var trees := PackedVector2Array()
		for tree in range(26):
			var angle := random.randf()*TAU
			var r := sqrt(random.randf())*(radius-18.0)
			trees.append(Vector2(cos(angle),sin(angle))*r)
		tree_layouts[key] = trees
	return tree_layouts[key]

# Historical frames predate terrain metadata.
func hill_footprint() -> Rect2:
	var hill: Dictionary = world.get("hill",{"x":1500,"y":1380,"radius":360})
	var radius := float(hill.radius)
	return Rect2(Vector2(hill.x,hill.y)-Vector2.ONE*radius,Vector2.ONE*radius*2.0)

func _classic_terrain() -> bool:
	return str(world.get("map", {}).get("size", "Classic")) == "Classic"

func _house_position(center: Vector2, index: int) -> Vector2:
	var limit := Vector2(2670, 1900) if _classic_terrain() else Vector2(map_width - 25, map_height - 25)
	return (center + _house_origin(index)).clamp(Vector2(5, 5), limit)


func _update_label_hover(point: Vector2) -> void:
	var next := -1
	for id in settlement_label_boxes:
		if settlement_label_boxes[id].has_point(point):
			next = int(id)
			break
	if next != hovered_settlement_label:
		hovered_settlement_label = next
		queue_redraw()

func _draw_settlement_labels(markets: Array) -> void:
	var font := ThemeDB.fallback_font
	var priority := {"Town": 3, "Village": 2, "Camp": 1}
	var ordered := markets.duplicate()
	ordered.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var first := int(priority.get(a.get("stage", ""), 0))
		var second := int(priority.get(b.get("stage", ""), 0))
		return first > second if first != second else int(a.get("id", 0)) < int(b.get("id", 0))
	)
	var occupied: Array[Rect2] = []
	for market in markets:
		var center := offset + Vector2(market.x, market.y) * zoom
		var radius := _label_clearance(market)
		occupied.append(Rect2(center - Vector2.ONE * radius, Vector2.ONE * radius * 2.0))
	for building in world.get("urban", {}).get("buildings", []):
		var center := offset + Vector2(building.x, building.y) * zoom
		var radius := maxf(5.0, 19.0 * zoom)
		occupied.append(Rect2(center - Vector2.ONE * radius, Vector2.ONE * radius * 2.0))
	var bounds := Rect2(Vector2(8, 66), size - Vector2(16, 107 if show_trade or show_resources else 74))
	for market in ordered:
		var anchor := offset + Vector2(market.x, market.y) * zoom
		if not Rect2(Vector2.ZERO, size).has_point(anchor):
			continue
		var title := str(market.name)
		var subtitle := "%s · %s people" % [market.get("stage", "Settlement"), int(market.population)]
		var width := maxf(font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x,
			font.get_string_size(subtitle, HORIZONTAL_ALIGNMENT_LEFT, -1, 11).x) + 18.0
		var gap := _label_clearance(market) + 8.0
		var candidates := [
			Vector2(0, -gap - 36), Vector2(-width - gap, -18),
			Vector2(gap, -18), Vector2(-width * 0.5, gap),
			Vector2(-width, -gap - 36), Vector2(0, gap),
		]
		for i in range(candidates.size()):
			var point: Vector2 = candidates[i]
			if i == 0:
				point.x -= width * 0.5
			var box := Rect2(anchor + point, Vector2(width, 36))
			if not bounds.encloses(box) or occupied.any(func(previous: Rect2) -> bool: return previous.intersects(box.grow(4))):
				continue
			occupied.append(box)
			var id := int(market.get("id", 0))
			settlement_label_boxes[id] = box
			var hovered := id == hovered_settlement_label
			draw_rect(box, Color(0.07, 0.13, 0.10, 0.92 if hovered else 0.42))
			if hovered:
				draw_rect(box, Color("b9a783"), false, 1.0)
			var title_at := box.position + Vector2(9, 14)
			var subtitle_at := box.position + Vector2(9, 29)
			draw_string(font, title_at + Vector2(1, 1), title, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color(0.05, 0.09, 0.06, 0.85))
			draw_string(font, subtitle_at + Vector2(1, 1), subtitle, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color(0.05, 0.09, 0.06, 0.85))
			draw_string(font, title_at, title, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color("fff0d1"))
			draw_string(font, subtitle_at, subtitle, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color("ecedd5"))
			break

func _draw_resource_marker(at: Vector2, kind: int, fraction: float, tint: Color) -> void:
	var r := 5.0 / zoom
	if kind in [4, 5, 6, 7]:
		draw_colored_polygon(PackedVector2Array([at + Vector2(0, -r), at + Vector2(r, 0),
			at + Vector2(0, r), at + Vector2(-r, 0)]), tint)
	else:
		draw_circle(at, r, Color("26382d"))
		draw_circle(at, r * 0.75, tint)
	draw_rect(Rect2(at + Vector2(-r, r + 2.0 / zoom), Vector2(2.0 * r, 2.0 / zoom)), Color("26382d"))
	draw_rect(Rect2(at + Vector2(-r, r + 2.0 / zoom), Vector2(2.0 * r * clampf(fraction, 0, 1), 2.0 / zoom)), Color("f3e7b9"))



# These layers are decorative. The snapshot still supplies every feature's position.
func _settlement_radius(market: Dictionary) -> float:
	var blocks := int(market.get("housing_blocks", 0))
	return 38.0 + minf(145.0, 23.0 * sqrt(float(blocks)))

func _label_clearance(market: Dictionary) -> float:
	var center := Vector2(market.x, market.y)
	var hill := hill_footprint()
	var radius := _settlement_radius(market)
	if _classic_terrain() and center.distance_to(hill.get_center()) < hill.size.x * 0.5:
		radius = maxf(radius, 106.0) * 1.08 + 6.0
	return maxf(15.0, radius * zoom)

func _draw() -> void:
	settlement_label_boxes.clear()
	terrain.queue_redraw()
	draw_set_transform(offset,0,Vector2(zoom,zoom))
	var markets: Array = world.get("markets", [])
	if show_zones:
		for zone in world.get("urban",{}).get("zones",[]):
			var color: Color = [Color("77b7ed"),Color("f4ce75"),Color("c49ae6")][int(zone.kind)]
			var at := Vector2(zone.x,zone.y)
			draw_circle(at,float(zone.radius),Color(color,0.22))
			draw_arc(at,float(zone.radius),0,TAU,48,color,2)
			draw_string(ThemeDB.fallback_font,at-Vector2(40,float(zone.radius)+5),["RESIDENTIAL","COMMERCIAL","INDUSTRIAL"][int(zone.kind)],HORIZONTAL_ALIGNMENT_LEFT,-1,12,color)
	for bank in world.get("banks",[]):
		var at := Vector2(bank.x,bank.y)
		if not _glyph("site-glyphs-xianxia/bank.png",at,44,44,1.0 if bank.ready else 0.55):
			var bank_color := Color("cbd2ad") if bank.ready else Color("b0a782")
			draw_rect(Rect2(at-Vector2(15,9),Vector2(30,20)),bank_color,bool(bank.ready),-1.0 if bank.ready else 2.0)
			if bank.ready:
				draw_colored_polygon(PackedVector2Array([at+Vector2(-18,-9),at+Vector2(0,-21),at+Vector2(18,-9)]),Color("6d8666"))
				for column in [-10,0,10]:
					draw_line(at+Vector2(column,-6),at+Vector2(column,8),Color("f5e7bc"),3)
		draw_string(ThemeDB.fallback_font,at+Vector2(-18,25),"Bank %s%%" % int(bank.rate_percent) if bank.ready else "Build bank",HORIZONTAL_ALIGNMENT_LEFT,-1,10,Color("f5e7bc"))
	for building in world.get("urban",{}).get("buildings",[]):
		var at := Vector2(building.x,building.y)
		var kind := int(building.kind)
		var line := int(building.line)
		var glyph_name: String = "home"
		if kind == 1:
			glyph_name = "shop-" + ["clothes","tools","furniture","butcher"][clampi(line,0,3)]
		elif kind == 2:
			glyph_name = "workshop-" + ["hunter","cobbler","blacksmith","carpenter"][clampi(line,0,3)]
		if not _glyph("site-glyphs-xianxia/"+glyph_name+".png",at,34,34,1.0 if building.ready else 0.55):
			var color: Color = [Color("92c4de"),Color("e0bd6d"),Color("b89aca")][int(building.kind)]
			draw_rect(Rect2(at-Vector2(11,8),Vector2(22,16)),color,bool(building.ready),-1.0 if building.ready else 2.0)
			if building.ready:
				draw_colored_polygon(PackedVector2Array([at+Vector2(-14,-8),at+Vector2(0,-18),at+Vector2(14,-8)]),color.darkened(0.25))
				draw_rect(Rect2(at+Vector2(-2,0),Vector2(5,8)),Color("55493c"))
		var caption: String = ["Home",["Clothes","Tools","Furniture","Butcher"][int(building.line)],["Hunter","Cobbler","Blacksmith","Carpenter"][int(building.line)]][int(building.kind)]
		if not building.ready:
			caption = "Clear site" if building.needs_clear else "Building"
		elif int(building.kind)==0 and building.owner==null and building.renter==null:
			caption = "For sale / rent"
		elif int(building.kind)>0 and building.owner==null:
			caption = "Lease available" if building.advertised else "Awaiting listing"
		if fidelity>1 or show_zones:
			draw_string(ThemeDB.fallback_font,at+Vector2(-20,23),caption,HORIZONTAL_ALIGNMENT_LEFT,-1,10,Color("f3e6c9"))
	for patch in world.get("patches",[]):
		var p := Vector2(patch.x,patch.y)
		var kind := int(patch.kind)
		if not show_resources and (int(patch.stock) == 0 or (fidelity == 1 and kind < 4)):
			continue
		var fraction: float = float(patch.stock) / maxf(1.0,float(patch.capacity))
		var colors := [Color("a95d76"),Color("365a38"),Color("d0ccb1"),Color("bce1df"),Color("edc65d"),Color("c8d6df"),Color("ba7650"),Color("8d949b"),Color("d7d18b"),Color("aab7b0"),Color("aab7b0"),Color("aab7b0"),Color("aab7b0")]
		if show_resources:
			_draw_resource_marker(p, kind, fraction, colors[kind])
			continue
		var radius := (5.0 if kind != 1 else 9.0) * (0.45 + fraction * 0.55)
		var glyph_path := ""
		if kind < PATCH_GLYPHS.size():
			glyph_path = "terrain-flora-xianxia/"+PATCH_GLYPHS[kind]+".png"
		elif kind in [4,5,6,7]:
			glyph_path = "site-glyphs-xianxia/"+MINE_GLYPHS[kind-4]+".png"
		if glyph_path == "" or not _glyph(glyph_path,p,2.2*radius*(0.6+0.4*fraction),2.2*radius*(0.6+0.4*fraction)):
			draw_circle(p+Vector2(2,3),radius,Color(0.1,0.2,0.1,0.18))
			draw_circle(p,radius,colors[kind])
			if kind in [4,5,6,7]:
				draw_rect(Rect2(p-Vector2(10,7),Vector2(20,14)),Color("535448"))
				draw_arc(p,9,PI,TAU,12,colors[kind],3)
			if kind == 1 and fraction > 0.2:
				draw_circle(p-Vector2(3,4),radius*0.7,Color("5d8750"))
		if kind in [4,5,6,7] and fidelity == 1:
			draw_string(ThemeDB.fallback_font,p+Vector2(-30,22),["Gold mine","Silver mine","Copper mine","Iron mine"][kind-4],HORIZONTAL_ALIGNMENT_LEFT,-1,11,colors[kind])
	for m in markets:
		var p := Vector2(m.x,m.y)
		if fidelity == 1:
			var radius := 7.0 / zoom
			match str(m.get("stage", "")):
				"Town":
					draw_rect(Rect2(p - Vector2.ONE * radius, Vector2.ONE * radius * 2), Color("293c32"))
					draw_rect(Rect2(p - Vector2.ONE * radius * 0.65, Vector2.ONE * radius * 1.3), Color("eed198"))
				"Village":
					draw_circle(p, radius, Color("293c32"))
					draw_circle(p, radius * 0.65, Color("dcb884"))
				_:
					draw_colored_polygon(PackedVector2Array([p + Vector2(0, -radius), p + Vector2(radius, radius), p + Vector2(-radius, radius)]), Color("634d37"))
			continue
		draw_circle(p,42,Color(0.95,0.87,0.62,0.24))
		draw_rect(Rect2(p-Vector2(16,11),Vector2(32,22)),Color("d6c698"))
		draw_rect(Rect2(p-Vector2(19,15),Vector2(38,13)),Color("a27254"))
		var blocks := int(m.housing_blocks)
		var view_rect := Rect2(-offset/zoom,size/zoom).grow(25.0)
		for j in range(blocks):
			var at := _house_position(p, j)
			if not view_rect.has_point(at):
				continue
			if not _glyph("site-glyphs-xianxia/home.png",at+Vector2(10,8),26,26):
				draw_rect(Rect2(at,Vector2(20,15)),Color("d1bf91"))
				draw_rect(Rect2(at-Vector2(1,2),Vector2(22,5)),Color("a07153"))
			if world.get("clock",{}).get("night",false):
				draw_rect(Rect2(at+Vector2(2,2),Vector2(3,3)),Color("f4c77a"))
		if world.get("clock",{}).get("night",false):
			draw_rect(Rect2(p+Vector2(-4,0),Vector2(7,5)),Color("ffda82"))
	for school in world.get("schools",[]):
		var p := Vector2(school.x,school.y)
		if not _glyph("site-glyphs-xianxia/school.png",p,42,42,1.0 if school.ready else 0.55):
			draw_rect(Rect2(p-Vector2(16,10),Vector2(32,20)),Color("d8d4b6") if school.ready else Color("85765b"))
			draw_rect(Rect2(p-Vector2(19,15),Vector2(38,8)),Color("526e9b"))
			draw_line(p+Vector2(-5,2),p+Vector2(0,4),Color("f4ecd0"),2)
			draw_line(p+Vector2(0,4),p+Vector2(5,2),Color("f4ecd0"),2)
		if fidelity>1:
			draw_string(ThemeDB.fallback_font,p+Vector2(-19,-22),"School" if school.ready else "Building school",HORIZONTAL_ALIGNMENT_LEFT,-1,11,Color("f4ecd0"))
	for tavern in world.get("taverns",[]):
		var p := Vector2(tavern.x,tavern.y)
		if not _glyph("site-glyphs-xianxia/tavern.png",p,42,42,1.0 if tavern.ready else 0.55):
			draw_rect(Rect2(p-Vector2(15,10),Vector2(30,20)),Color("d9bc87") if tavern.ready else Color("85765b"))
			draw_rect(Rect2(p-Vector2(18,14),Vector2(36,8)),Color("913f39"))
			draw_circle(p+Vector2(7,2),3,Color("ecc864"))
		if fidelity>1:
			draw_string(ThemeDB.fallback_font,p+Vector2(-18,-20),"Tavern %s/32" % int(tavern.visitors) if tavern.ready else "Building tavern",HORIZONTAL_ALIGNMENT_LEFT,-1,10,Color("f7e2b2"))
	for region in world.get("fertility",[]):
		var p := Vector2(region.x,region.y)
		draw_rect(Rect2(p-Vector2(region.width,region.height)*0.5,Vector2(region.width,region.height)),Color(0.40,0.58,0.30,0.18))
	for farm in world.get("farms",[]):
		var p := Vector2(farm.x,farm.y)
		var color := Color("c8c15f") if farm.mature else Color("739550")
		if not _glyph("site-glyphs-xianxia/farm.png",p,44,44):
			draw_rect(Rect2(p-Vector2(18,18),Vector2(36,36)),Color("5d8076"))
			for row in range(6):
				draw_line(p+Vector2(-15,-14+row*5),p+Vector2(15,-14+row*5),color,2)
		if fidelity > 1:
			draw_string(ThemeDB.fallback_font,p+Vector2(-18,-22),"Rice / %s days" % int(farm.days_until_mature),HORIZONTAL_ALIGNMENT_LEFT,-1,10,Color("eee5b4"))
	for board in world.get("job_boards",[]):
		var p := Vector2(board.x,board.y)
		if not _glyph("site-glyphs-xianxia/jobboard.png",p,24,24):
			draw_rect(Rect2(p-Vector2(6,5),Vector2(12,9)),Color("634b34"))
			draw_rect(Rect2(p-Vector2(4,3),Vector2(8,5)),Color("ead7a6"))
			draw_line(p+Vector2(-4,3),p+Vector2(-4,9),Color("634b34"),2)
			draw_line(p+Vector2(4,3),p+Vector2(4,9),Color("634b34"),2)
		if int(board.nearby_bodies)>0:
			draw_circle(p+Vector2(6,-5),3,Color("dd8a65"))
		if fidelity == 1:
			draw_string(ThemeDB.fallback_font,p+Vector2(7,12),"Jobs %s / %s c" % [int(board.vacancies),int(board.reward)],HORIZONTAL_ALIGNMENT_LEFT,-1,9,Color("f7e2b2"))
	for graveyard in world.get("graveyards",[]):
		var p := Vector2(graveyard.x,graveyard.y)
		if not _glyph("site-glyphs-xianxia/graveyard.png",p,44,44,1.0 if graveyard.ready else 0.55):
			draw_rect(Rect2(p-Vector2(17,17),Vector2(38,38)),Color("858a70"))
			draw_rect(Rect2(p-Vector2(18,18),Vector2(40,40)),Color("c2bca0"),false,1.5)
			var graves := mini(int(graveyard.buried_count),64)
			for j in range(graves):
				var at := p+Vector2(float(j%8)*4.0-12,float(j/8)*4.0-12)
				draw_rect(Rect2(at,Vector2(2,3)),Color("c3c9ba"))
		if not graveyard.ready:
			draw_line(p-Vector2(12,0),p+Vector2(15,0),Color("d0ad76"),2)
		draw_string(ThemeDB.fallback_font,p+Vector2(-18,-23),"Graveyard %s/%s" % [int(graveyard.buried_count),int(graveyard.capacity)],HORIZONTAL_ALIGNMENT_LEFT,-1,10,Color("eee7d2"))
	for shelter in world.get("shelters",[]):
		if shelter.get("camp",false) and int(shelter.get("capacity",0))>0:
			var p := Vector2(shelter.x,shelter.y)
			if fidelity > 1:
				draw_arc(p,13.0,0,TAU,24,Color("e8dcb4"),2.5)
			if not _glyph("site-glyphs-xianxia/shelter.png",p,30,30):
				draw_colored_polygon(PackedVector2Array([p+Vector2(-9,6),p+Vector2(0,-9),p+Vector2(9,6)]),Color("c6a87c"))
				draw_line(p+Vector2(0,-5),p+Vector2(0,6),Color("614e3a"),2.0)
			if fidelity == 1:
				draw_string(ThemeDB.fallback_font,p+Vector2(-15,16),"%s/%s beds" % [int(shelter.reserved),int(shelter.capacity)],HORIZONTAL_ALIGNMENT_LEFT,-1,9,Color("f6e6b9"))
	var convoy_draw: Array = []
	var rally_markers: Dictionary = {}
	for convoy in world.get("convoys",[]) if fidelity == 1 else []:
		if convoy.departed:
			convoy_draw.append(convoy)
		else:
			var spot := Vector2(convoy.x,convoy.y)
			if rally_markers.has(spot):
				rally_markers[spot].members += int(convoy.members)
			else:
				var marker: Dictionary = convoy.duplicate()
				rally_markers[spot] = marker
				convoy_draw.append(marker)
	for convoy in convoy_draw:
		var p := Vector2(convoy.x,convoy.y)
		var color := Color("f4d88d") if convoy.departed else Color("e4e4be")
		draw_rect(Rect2(p-Vector2(6,4),Vector2(12,8)),color,convoy.departed,-1.0 if convoy.departed else 1.5)
		draw_circle(p+Vector2(-4,5),2,Color("443c30"))
		draw_circle(p+Vector2(4,5),2,Color("443c30"))
		draw_string(ThemeDB.fallback_font,p+Vector2(8,-4),str(int(convoy.members)),HORIZONTAL_ALIGNMENT_LEFT,-1,10,color)
	var animal_view := Rect2(-offset/zoom,size/zoom).grow(12.0)
	for animal in world.get("wildlife",{}).get("animals",[]):
		var at := Vector2(animal.x,animal.y)
		if not animal_view.has_point(at):
			continue
		if fidelity==1:
			draw_circle(at,3.3 if int(animal.species)==1 else 2.2,Color("dcba80") if int(animal.species)==1 else Color("f0e8d5"))
		else:
			var scale_animal := 0.6 if animal.young else 1.0
			var bob := sin(animation_time*10.0+float(animal.id))*0.5 if fidelity==3 and int(animal.action)!=3 else 0.0
			var species_col := int(animal.species)
			var stance_row := 0
			match int(animal.action):
				0, 1: stance_row = 3 # Feeding
				2: stance_row = 4 # Fleeing
				4, 5:
					if fidelity == 3:
						stance_row = 1 + int(animation_time*10.0 + float(animal.id)) % 2
			var animal_tex := _tex("animal-table-xianxia/atlas.png")
			draw_set_transform(offset+at*zoom,float(animal.heading),Vector2.ONE*zoom*scale_animal)
			if animal_tex != null and species_col < 9:
				draw_texture_rect_region(animal_tex,Rect2(-9,-9,18,18),Rect2(species_col*64,stance_row*64,64,64))
			else:
				var deer := int(animal.species)==1
				draw_circle(Vector2.ZERO,4.0 if deer else 2.8,Color("af8558") if deer else Color("e5ddc8"))
				draw_circle(Vector2(3.5,0),2.0 if deer else 1.6,Color("ccaa78") if deer else Color("f4ecdc"))
				if deer:
					for side in [-1,1]:
						draw_line(Vector2(1,side*2),Vector2(-2+bob,side*5),Color("65513c"),1)
						draw_line(Vector2(4,side),Vector2(7,side*4),Color("705c42"),1)
						draw_line(Vector2(5.5,side*2.5),Vector2(8,side*2.5),Color("705c42"),1)
				else:
					draw_line(Vector2(3,1),Vector2(7,2+bob),Color("eee3d0"),1.5)
					draw_line(Vector2(3,-1),Vector2(7,-2+bob),Color("eee3d0"),1.5)
				draw_circle(Vector2(4.2,-0.7),0.4,Color("302a24"))
			draw_set_transform(offset,0,Vector2(zoom,zoom))
		if int(animal.id)==selected_animal:
			draw_arc(at,8.0,0,TAU,20,Color("ffedaa"),1.0)
	if site_focus and has_focus_target:
		draw_arc(focus_target,22.0,0,TAU,32,Color("ffedaa"),1.5)
	if selected_index >= 0:
		draw_arc(points[selected_index],4.5 if fidelity > 1 else 11.0,0,TAU,24,Color("ffedaa"),0.4 if fidelity > 1 else 2.0)
	if fidelity == 1 and _classic_terrain():
		draw_string(ThemeDB.fallback_font,Vector2(25,31),"WOODLAND GROVES",HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("deebc7"))
		draw_string(ThemeDB.fallback_font,Vector2(2733,93),"COAST",HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("e2f2e6"))
	draw_set_transform(Vector2.ZERO)
	if fidelity == 1:
		_draw_settlement_labels(markets)
	var legend: Array[String] = []
	if show_trade:
		legend.append("Teal lines: cumulative road cargo")
	if show_resources:
		legend.append("Resource bars: stock remaining · Diamonds: mines")
	if not legend.is_empty():
		var legend_text := "   •   ".join(legend)
		var width := ThemeDB.fallback_font.get_string_size(legend_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 12).x + 20.0
		draw_style_box(hud_style, Rect2(12, size.y - 37, width, 25))
		draw_string(ThemeDB.fallback_font, Vector2(22, size.y - 20), legend_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color("f7eed0"))
	draw_style_box(hud_style,Rect2(12,12,360,49))
	draw_string(ThemeDB.fallback_font,Vector2(23,32),"LEVEL %s / %s / %.1fx" % [fidelity,["OVERVIEW","INDIVIDUAL FEATURES","ANIMATED INDIVIDUALS"][fidelity-1],magnification],HORIZONTAL_ALIGNMENT_LEFT,-1,13,Color("f7eed0"))
	draw_string(ThemeDB.fallback_font,Vector2(23,49),"Wheel: zoom   Left drag: pan / Click: select   Detailed people: %s" % draw_indices.size(),HORIZONTAL_ALIGNMENT_LEFT,-1,11,Color("b7c7ad"))



func _clamp_camera() -> void:
	if magnification <= 1.01:
		camera_center = Vector2(map_width,map_height)*0.5
	else:
		camera_center.x = clampf(camera_center.x,0,map_width)
		camera_center.y = clampf(camera_center.y,0,map_height)

func set_fidelity(level: int) -> void:
	magnification = [1.0,3.0*(map_width/1024.0),8.0*(map_width/1024.0)][clampi(level-1,0,2)]
	if level > 1:
		for i in range(ids.size()):
			if ids[i] == selected:
				camera_center = points[i]
				break
	_clamp_camera()
	_resize()

func _update_detail_indices() -> void:
	draw_indices.clear()
	for i in range(meshes.size()):
		meshes[i].visible = fidelity == 1 and i == 0
	detail_mesh.visible = fidelity > 1
	if fidelity == 1:
		if overview_dirty:
			_refresh_overview()
		return
	var rect := Rect2(-offset/zoom,size/zoom).grow(12.0)
	var low := Vector2i(floori(rect.position.x/CELL),floori(rect.position.y/CELL))
	var high := Vector2i(floori(rect.end.x/CELL),floori(rect.end.y/CELL))
	# Always retain the inspected person in crowded views.
	if selected_index >= 0 and hidden_flags[selected_index] == 0 and rect.has_point(points[selected_index]):
		draw_indices.append(selected_index)
	for y in range(maxi(0,low.y),mini(ceili(map_height/CELL),high.y)+1):
		for x in range(maxi(0,low.x),mini(ceili(map_width/CELL),high.x)+1):
			for i: int in spatial.get(Vector2i(x,y),[]):
				if i != selected_index and hidden_flags[i] == 0 and rect.has_point(points[i]):
					draw_indices.append(i)
					if draw_indices.size() >= 2500:
						_refresh_details()
						return
	_refresh_details()

func _refresh_details() -> void:
	detail_buffer.resize(draw_indices.size()*12)
	for rank in range(draw_indices.size()):
		var i: int = draw_indices[rank]
		var at := rank*12
		var mode := 0
		if healths[i] == 0:
			mode = 3
		elif sleeping_flags[i] > 0:
			mode = 5
		elif fidelity == 3 and moving[i] > 0:
			mode = 1
		elif actions[i] in [1,4,6,8,11,12,15,17,18]:
			mode = 2 if fidelity == 3 else 4
		var previous_delta := draw_points[i]-points[i] if fidelity == 3 else Vector2.ZERO
		var body_scale := 0.30 if sleeping_flags[i] > 0 else (0.50 if child_flags[i] > 0 else 1.0)
		detail_buffer[at] = body_scale
		detail_buffer[at+3] = points[i].x
		detail_buffer[at+5] = body_scale
		detail_buffer[at+7] = points[i].y
		detail_buffer[at+8] = float(ids[i]%16+(mode+8*rested_flags[i]+16*carrying_flags[i])*16)
		detail_buffer[at+9] = previous_delta.x/body_scale
		detail_buffer[at+10] = previous_delta.y/body_scale
		detail_buffer[at+11] = float(ids[i]%31)*0.2
	var mm := detail_mesh.multimesh
	if mm.instance_count != draw_indices.size():
		mm.instance_count = draw_indices.size()
	mm.buffer = detail_buffer

func _process(delta: float) -> void:
	if follow_selection and has_focus_target and camera_center.distance_to(focus_target)>0.05:
		camera_center = camera_center.lerp(focus_target,minf(1.0,delta*10.0))
		_resize()
	if camera_dirty:
		camera_dirty = false
		_update_detail_indices()
		detail_mesh.position = offset
		detail_mesh.scale = Vector2(zoom,zoom)
		queue_redraw()
	if fidelity != 3:
		return
	snapshot_age += delta
	if not world.get("paused",true):
		shader_clock += delta
	detail_material.set_shader_parameter("clock",shader_clock)
	detail_material.set_shader_parameter("blend_amount",minf(snapshot_age/snapshot_interval,1.0))

func _build_detail_renderer() -> void:
	detail_mesh = MultiMeshInstance2D.new()
	detail_mesh.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	detail_mesh.texture = _tex("villager-atlas-v2/atlas.png")
	if detail_mesh.texture == null:
		atlas_view = SubViewport.new()
		atlas_view.size = Vector2i(1024,1280)
		atlas_view.transparent_bg = true
		atlas_view.disable_3d = true
		atlas_view.render_target_update_mode = SubViewport.UPDATE_ONCE
		atlas_view.canvas_item_default_texture_filter = Viewport.DEFAULT_CANVAS_ITEM_TEXTURE_FILTER_NEAREST
		atlas_view.add_child(preload("res://person_atlas.gd").new())
		add_child(atlas_view)
		detail_mesh.texture = atlas_view.get_texture()
	detail_mesh.multimesh = MultiMesh.new()
	detail_mesh.multimesh.transform_format = MultiMesh.TRANSFORM_2D
	detail_mesh.multimesh.use_custom_data = true
	var quad := QuadMesh.new()
	quad.size = Vector2(10,12)
	detail_mesh.multimesh.mesh = quad
	detail_material = ShaderMaterial.new()
	detail_material.shader = preload("res://people.gdshader")
	detail_mesh.material = detail_material
	add_child(detail_mesh)

func _house_origin(house: int) -> Vector2:
	var original := house
	var ring := 0
	while house >= 12+ring*6:
		house -= 12+ring*6
		ring += 1
	var angle := float(house)*TAU/float(12+ring*6)+float(ring)*0.47+(float(original*53%17)-8.0)*0.008
	return Vector2(cos(angle),sin(angle))*(140.0+float(ring)*42.0+float(original*37%19)-9.0)
