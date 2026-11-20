extends Node2D
# Ground, coast, hill, woodland, clearings, settlement ground and roads.
# Drawn behind the field so sites, wildlife and people stay on top.
var field: Control
var ground := Ground.new()
var map_key := ""
var requested_key := ""
var explored_ms := 0
var fog_texture: Texture2D
var fog_rect := Rect2()
var raster_cell := 0.0
var raster_cols := 0
var raster_rows := 0
var raster_classes := PackedByteArray()
var raster_regions := PackedByteArray()
var region_labels: Array = []
# Shore wave accents, river courses and region borders, each bucketed by
# 256-unit regions so only visible strokes draw.
var shore_buckets := {}
var river_buckets := {}
var border_buckets := {}

# Smoothly filtered generated-map ground, drawn before the layer's own strokes.
class Ground extends Node2D:
	var field: Control
	var texture: Texture2D
	func _ready() -> void:
		show_behind_parent = true
		texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO,field.size),Color("15231d"))
		draw_set_transform(field.offset,0,Vector2(field.zoom,field.zoom))
		var map := Rect2(0, 0, field.map_width, field.map_height)
		if texture != null and not field._classic_terrain():
			draw_texture_rect(texture, map, false)
		else:
			draw_rect(map, Color("93a575"))

# Mist over unexplored chunks, above woodland and roads but below sites.
class Fog extends Node2D:
	var layer: Node2D
	func _ready() -> void:
		texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	func _draw() -> void:
		var field: Control = layer.field
		if layer.fog_texture == null or field._classic_terrain():
			return
		draw_set_transform(field.offset,0,Vector2(field.zoom,field.zoom))
		draw_texture_rect(layer.fog_texture, layer.fog_rect, false)

var fog := Fog.new()

func _ready() -> void:
	show_behind_parent = true
	ground.field = field
	add_child(ground)
	fog.layer = self
	add_child(fog)

# True once per generated map so the viewer asks the backend for its rasters.
func wants_map_layers() -> bool:
	if field._classic_terrain():
		return false
	var key := JSON.stringify(field.world.get("map", {}))
	if key == map_key or key == requested_key:
		return false
	requested_key = key
	return true

# Every two seconds after the terrain arrives, refresh which chunks are explored.
func wants_explored() -> bool:
	var now := Time.get_ticks_msec()
	if map_key == "" or map_key != JSON.stringify(field.world.get("map", {})) or now-explored_ms < 2000:
		return false
	explored_ms = now
	return true

func set_map_layers(layers: Dictionary) -> void:
	var key := JSON.stringify(layers.get("map", {}))
	var terrain: Dictionary = layers.get("terrain", {})
	if not terrain.is_empty():
		ground.texture = ImageTexture.create_from_image(terrain_image(terrain))
		raster_cols = int(terrain.cols)
		raster_rows = int(terrain.rows)
		raster_cell = float(terrain.cell)
		raster_classes = Marshalls.base64_to_raw(terrain.classes)
		raster_regions = Marshalls.base64_to_raw(terrain.get("regions", ""))
		region_labels = terrain.get("region_labels", [])
		_build_shore()
		_build_rivers(terrain.get("rivers", []))
		_build_borders()
		map_key = key
		requested_key = ""
	var explored: Dictionary = layers.get("explored", {})
	if key == map_key and not explored.is_empty():
		fog_texture = ImageTexture.create_from_image(fog_image(explored))
		fog_rect = Rect2(0, 0, int(explored.cols)*float(explored.chunk), int(explored.rows)*float(explored.chunk))
	# Swapped textures free the old RIDs; every item with recorded draw
	# commands against them must re-record this pass or it draws blank.
	ground.queue_redraw()
	fog.queue_redraw()
	field.minimap.queue_redraw()
	queue_redraw()

# Unexplored chunks under a pale mist; linear filtering feathers the borders.
static func fog_image(explored: Dictionary) -> Image:
	var cols := int(explored.cols)
	var rows := int(explored.rows)
	var known := Marshalls.base64_to_raw(explored.known)
	assert(known.size() == cols*rows)
	var pixels := PackedByteArray()
	pixels.resize(cols*rows*4)
	for at in range(cols*rows):
		pixels[at*4] = 214
		pixels[at*4+1] = 222
		pixels[at*4+2] = 216
		pixels[at*4+3] = 0 if known[at] else 222
	return Image.create_from_data(cols, rows, false, Image.FORMAT_RGBA8, pixels)

const TERRAIN_COLORS := [Color("35606d"), Color("9a9682"), Color("71866a"), Color("62804f"), Color("a5ad6c"), Color("93a575"), Color("4f8f9b")]

# One pixel per generator cell: class colour, light from the north-west, sandy
# shores and a faint jade cast where qi gathers.
static func terrain_image(terrain: Dictionary) -> Image:
	var cols := int(terrain.cols)
	var rows := int(terrain.rows)
	var classes := Marshalls.base64_to_raw(terrain.classes)
	var elevation := Marshalls.base64_to_raw(terrain.elevation)
	var qi := Marshalls.base64_to_raw(terrain.qi)
	assert(classes.size() == cols*rows and elevation.size() == cols*rows and qi.size() == cols*rows)
	var pixels := PackedByteArray()
	pixels.resize(cols*rows*4)
	for j in range(rows):
		for i in range(cols):
			var at := j*cols+i
			var kind := mini(int(classes[at]), TERRAIN_COLORS.size()-1)
			var e := elevation[at]/255.0
			var color: Color = TERRAIN_COLORS[kind]
			if kind == 0 or kind == 6:
				color = color.lerp(Color("5d8c8f"), clampf(e/0.34, 0.0, 1.0)*0.8)
			else:
				var nw := elevation[maxi(j-1,0)*cols+maxi(i-1,0)]/255.0
				var se := elevation[mini(j+1,rows-1)*cols+mini(i+1,cols-1)]/255.0
				var light := clampf((se-nw)*7.0, -0.22, 0.22)
				color = color.lightened(light) if light > 0.0 else color.darkened(-light)
				if kind == 1:
					color = color.lerp(Color("c9c6b4"), clampf((e-0.78)*4.0, 0.0, 0.6))
				if e < 0.38:
					color = color.lerp(Color("c7bd91"), clampf((0.38-e)/0.04, 0.0, 1.0)*0.7)
				color = color.lerp(Color("8fd3c4"), qi[at]/255.0*0.35)
			pixels[at*4] = color.r8
			pixels[at*4+1] = color.g8
			pixels[at*4+2] = color.b8
			pixels[at*4+3] = 255
	return Image.create_from_data(cols, rows, false, Image.FORMAT_RGBA8, pixels)

# Short waterline dashes on water cells that touch land; they read as wave
# detail when the map is zoomed in and stay hidden far out.
func _build_shore() -> void:
	shore_buckets.clear()
	if raster_cols < 3 or raster_classes.size() != raster_cols*raster_rows:
		return
	for j in range(1, raster_rows - 1):
		for i in range(1, raster_cols - 1):
			if int(raster_classes[j*raster_cols+i]) != 0:
				continue
			for dir in [Vector2(1,0), Vector2(-1,0), Vector2(0,1), Vector2(0,-1)]:
				var ni := i + int(dir.x)
				var nj := j + int(dir.y)
				if int(raster_classes[nj*raster_cols+ni]) == 0:
					continue
				var edge: Vector2 = Vector2(i + 0.5, j + 0.5)*raster_cell + dir*raster_cell*0.38
				var n: Vector2 = dir.orthogonal()*raster_cell*0.30
				var key := Vector2i(edge / 256.0)
				if not shore_buckets.has(key):
					shore_buckets[key] = PackedVector2Array()
				shore_buckets[key].append_array([edge - n, edge + n])
				break

# River course strokes from the sim's traced watercourses. Each bucket
# holds a parallel (points, widths) pair keyed by start cell.
func _build_rivers(rivers: Array) -> void:
	river_buckets.clear()
	for river in rivers:
		var pts: Array = river.get("p", [])
		var w := float(river.get("w", 1.0)) * 2.0
		for k in range(pts.size() - 1):
			var a := Vector2(pts[k][0], pts[k][1])
			var b := Vector2(pts[k + 1][0], pts[k + 1][1])
			var key := Vector2i(a / 256.0)
			if not river_buckets.has(key):
				river_buckets[key] = [PackedVector2Array(), PackedFloat32Array()]
			river_buckets[key][0].append_array([a, b])
			river_buckets[key][1].append(w)

# Region borders follow cell edges where neighbouring land cells belong to
# different territories; only meaningful zoomed out.
func _build_borders() -> void:
	border_buckets.clear()
	var n := raster_cols * raster_rows
	if raster_regions.size() != n or n == 0:
		return
	for j in range(1, raster_rows - 1):
		for i in range(1, raster_cols - 1):
			var id := int(raster_regions[j*raster_cols+i])
			if id == 255:
				continue
			var c := Vector2(i + 0.5, j + 0.5)*raster_cell
			var right := int(raster_regions[j*raster_cols+i+1])
			var down := int(raster_regions[(j+1)*raster_cols+i])
			var edge := PackedVector2Array()
			if right != 255 and right != id:
				edge.append_array([c + Vector2(0.5, -0.45)*raster_cell, c + Vector2(0.5, 0.45)*raster_cell])
			if down != 255 and down != id:
				edge.append_array([c + Vector2(-0.45, 0.5)*raster_cell, c + Vector2(0.45, 0.5)*raster_cell])
			if edge.is_empty():
				continue
			var key := Vector2i(c / 256.0)
			if not border_buckets.has(key):
				border_buckets[key] = PackedVector2Array()
			border_buckets[key].append_array(edge)

func _draw_strokes(buckets: Dictionary, view: Rect2, color: Color, width: float) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var lines: PackedVector2Array = buckets.get(Vector2i(bx, by), PackedVector2Array())
			for s in range(0, lines.size(), 2):
				draw_line(lines[s], lines[s + 1], color, width, true)

func _draw_rivers(view: Rect2) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var bucket: Array = river_buckets.get(Vector2i(bx, by), [])
			if bucket.is_empty():
				continue
			var lines: PackedVector2Array = bucket[0]
			var widths: PackedFloat32Array = bucket[1]
			for s in range(0, lines.size(), 2):
				draw_line(lines[s], lines[s + 1], Color("5d97a6"), widths[s/2], true)

func _draw_region_labels(view: Rect2) -> void:
	for label in region_labels:
		var at := Vector2(label.x, label.y)
		if not view.has_point(at):
			continue
		draw_string(ThemeDB.fallback_font, at + Vector2(-60, 0), str(label.n),
			HORIZONTAL_ALIGNMENT_LEFT, -1, 40.0, Color(0.13, 0.19, 0.16, 0.55))

func _draw_shore(view: Rect2) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var lines: PackedVector2Array = shore_buckets.get(Vector2i(bx, by), PackedVector2Array())
			for s in range(0, lines.size(), 2):
				draw_line(lines[s], lines[s + 1], Color(0.76, 0.88, 0.84, 0.38), 1.6, true)

# Fidelity-3 water motion: a pale bead creeps downstream along each river
# stroke and the shore dashes breathe, both on the field's shader_clock so a
# paused world holds the frame.
func _draw_river_flow(view: Rect2) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var bucket: Array = river_buckets.get(Vector2i(bx, by), [])
			if bucket.is_empty():
				continue
			var lines: PackedVector2Array = bucket[0]
			var widths: PackedFloat32Array = bucket[1]
			for s in range(0, lines.size(), 2):
				var t := fposmod(field.shader_clock * 0.30 + float(s / 2) * 0.19, 1.0)
				var bead := lines[s].lerp(lines[s + 1], t)
				draw_circle(bead, widths[s / 2] * 0.45 + 1.2, Color(0.82, 0.95, 0.93, 0.45))

func _draw_shore_anim(view: Rect2) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var lines: PackedVector2Array = shore_buckets.get(Vector2i(bx, by), PackedVector2Array())
			for s in range(0, lines.size(), 2):
				var pulse := 0.24 + 0.20 * sin(field.shader_clock * 2.1 + float(s) * 0.61)
				draw_line(lines[s], lines[s + 1], Color(0.76, 0.88, 0.84, pulse), 1.8, true)

func _glyph(rel_path: String, at: Vector2, w: float, h: float, alpha := 1.0) -> bool:
	var t: Texture2D = field._tex(rel_path)
	if t == null:
		return false
	draw_texture_rect(t, Rect2(at - Vector2(w,h)*0.5, Vector2(w,h)), false,
		Color(1,1,1,alpha))
	return true

# Same ladders as field.gd, but drawing on this canvas item: draw calls must
# be recorded by the node whose _draw is running.
func _anim(id: String, at: Vector2, w: float, h: float, alpha := 1.0) -> bool:
	var t: Texture2D = field._tex("lod/anim/%s.png" % id)
	if t == null:
		return false
	var frames := maxi(1, t.get_width() / 64)
	var frame := int(field.shader_clock * 10.0) % frames
	draw_texture_rect_region(t, Rect2(at - Vector2(w,h)*0.5, Vector2(w,h)),
		Rect2(frame * 64, 0, 64, 64), Color(1,1,1,alpha))
	return true

func _dot(id: String, at: Vector2, size: float, alpha := 1.0) -> bool:
	return _glyph("lod/dots/%s.png" % id, at, size, size, alpha)

func _asset(id: String, static_path: String, at: Vector2, w: float, h: float, alpha := 1.0) -> bool:
	if field.fidelity == 3 and _anim(id, at, w, h, alpha):
		return true
	if static_path != "":
		return _glyph(static_path, at, w, h, alpha)
	return _glyph("lod/l2/%s.png" % id, at, w, h, alpha)

func _draw() -> void:
	ground.queue_redraw()
	fog.queue_redraw()
	draw_set_transform(field.offset,0,Vector2(field.zoom,field.zoom))
	var forest_view := Rect2(-field.offset / field.zoom, field.size / field.zoom)
	if field._classic_terrain():
		_draw_coast_art(forest_view)
		_draw_hill_art(forest_view)
	else:
		_draw_rivers(forest_view)
		if field.fidelity == 3:
			_draw_river_flow(forest_view)
		if field.fidelity == 1:
			_draw_shore(forest_view)
		else:
			_draw_strokes(border_buckets, forest_view, Color(0.20, 0.24, 0.18, 0.45), 2.5)
			_draw_region_labels(forest_view)
			if field.fidelity == 3:
				_draw_shore_anim(forest_view)
	var markets: Array = field.world.get("markets", [])
	var forests: Array = field.world.get("wildlife", {}).get("forests", [])
	# Paint the woodland floor before canopies so neighboring groves read together.
	for id in range(forests.size()):
		var forest: Dictionary = forests[id]
		var center := Vector2(forest.x, forest.y)
		var radius := float(forest.radius)
		if not forest_view.grow(radius * 1.25).has_point(center):
			continue
		draw_colored_polygon(_terrain_shape(center, Vector2(radius * 1.22, radius * 1.14), float(id)), Color(0.32, 0.48, 0.32, 0.30))
		draw_colored_polygon(_terrain_shape(center, Vector2(radius, radius * 0.93), float(id)), Color("617f53"))
	for id in range(forests.size()):
		var forest: Dictionary = forests[id]
		var centre := Vector2(forest.x,forest.y)
		if not forest_view.grow(float(forest.radius)).has_point(centre):
			continue
		var trees: PackedVector2Array = field._tree_points(id,float(forest.radius))
		for tree in range(0,trees.size(),3 if field.fidelity==1 else 1):
			var at := centre+trees[tree]
			var side := 22.0+float(tree%4)*2.0
			var tree_id: String = field.TREE_GLYPHS[(id*3+tree)%field.TREE_GLYPHS.size()]
			if field.fidelity == 3 and _anim(tree_id,at,side,side):
				continue
			if field.fidelity > 1 and _glyph("terrain-flora-xianxia/"+tree_id+".png",at,side,side):
				continue
			var crown := 9.0 + float(tree % 5)
			draw_circle(at + Vector2(4, 5), crown, Color(0.10, 0.22, 0.16, 0.22), true, -1.0, true)
			draw_circle(at, crown, Color("365c43"), true, -1.0, true)
			draw_circle(at + Vector2(-2, -3), crown * 0.76, Color("507448"), true, -1.0, true)
			draw_circle(at + Vector2(-4, -5), crown * 0.44, Color("739055"), true, -1.0, true)
	for clearing in field.world.get("urban",{}).get("cleared",[]):
		draw_circle(Vector2(clearing[0],clearing[1]),float(clearing[2]),Color("93a575"))
	for grass in field.world.get("wildlife",{}).get("grass",[]):
		var fraction := float(grass.stock)/maxf(1.0,float(grass.capacity))
		var gpos := Vector2(grass.x,grass.y)
		var done := false
		if field.fidelity == 1:
			done = _dot("grass",gpos,9.0/field.zoom)
		else:
			done = _asset("grass","lod/l2/grass.png",gpos,18,18,1.0 if fraction>0.25 else 0.45)
		if not done:
			draw_circle(gpos,7.0,Color("78a65a") if fraction>0.25 else Color("998761"))
	_draw_settlement_ground(markets, forest_view)
	_draw_roads(markets)
	_draw_ports()
	_draw_ships()

func _terrain_shape(center: Vector2, radii: Vector2, phase: float) -> PackedVector2Array:
	var points := PackedVector2Array()
	for i in range(24):
		var angle := TAU * i / 24.0
		var edge := 0.91 + 0.055 * sin(angle * 3.0 + phase) + 0.035 * cos(angle * 5.0 - phase)
		points.append(center + Vector2.from_angle(angle) * radii * edge)
	return points

func _draw_hill_art(view: Rect2) -> void:
	var hill: Rect2 = field.hill_footprint()
	if not view.intersects(hill):
		return
	var center := hill.get_center()
	var radius := hill.size.x * 0.5
	var skirt := _terrain_shape(center, Vector2.ONE * radius, 1.3)
	draw_colored_polygon(skirt, Color("838d70"))
	for layer in range(4):
		var scale := 0.88 - layer * 0.16
		var at := center + Vector2(-0.075, -0.12) * radius * layer
		var contour := _terrain_shape(at, Vector2(1.0, 0.91) * radius * scale, 1.3)
		draw_colored_polygon(contour, [Color("98a084"), Color("a8ad94"), Color("b7bba4"), Color("c5c9b0")][layer])
		contour.append(contour[0])
		draw_polyline(contour, Color("798570"), 2.0, true)
	for side in range(5):
		var at := center + Vector2(0.03 + side * 0.105, 0.51 - side * 0.09) * radius
		var face := PackedVector2Array([at, at + Vector2(0.07, -0.13) * radius,
			at + Vector2(0.14, -0.16) * radius, at + Vector2(0.10, 0.01) * radius])
		draw_colored_polygon(face, Color("777f70").lerp(Color("9fa58c"), 0.12 * side))

func _draw_coast_art(view: Rect2) -> void:
	if field.map_width <= 2700.0 or view.end.x < 2635.0:
		return
	draw_rect(Rect2(2700, 0, field.map_width - 2700, field.map_height), Color("477986"))
	var shore := PackedVector2Array([Vector2(2700, 0), Vector2(2700, field.map_height)])
	var waterline := PackedVector2Array()
	for i in range(49):
		var y: float = field.map_height * i / 48.0
		var x := 2672.0 + 13.0 * sin(y / 130.0) + 5.0 * sin(y / 44.0)
		waterline.append(Vector2(x, y))
	for i in range(waterline.size() - 1, -1, -1):
		shore.append(waterline[i])
	draw_colored_polygon(shore, Color("c7bd91"))
	draw_polyline(waterline, Color("dacb9f"), 3.0, true)
	for y in range(30, int(field.map_height), 78):
		var x := 2720.0 + 10.0 * sin(float(y) / 100.0)
		var foam := 0.19
		if field.fidelity == 3:
			foam = 0.10 + 0.13 * sin(field.shader_clock * 1.9 + float(y) * 0.05)
		draw_line(Vector2(x, y), Vector2(x + 30, y + 6), Color(0.76, 0.88, 0.84, foam), 2.0, true)

func _draw_settlement_ground(markets: Array, view: Rect2) -> void:
	for market in markets:
		var center := Vector2(market.x, market.y)
		var radius: float = field._settlement_radius(market)
		if not view.grow(radius).has_point(center):
			continue
		var hill: Rect2 = field.hill_footprint()
		var phase := float(market.get("id", 0)) * 0.8
		if field._classic_terrain() and center.distance_to(hill.get_center()) < hill.size.x * 0.5:
			# Existing hillside villages occupy a cut bench with a visible retaining edge.
			var bench := Vector2(maxf(radius, 106.0), maxf(radius * 0.72, 63.0))
			draw_colored_polygon(_terrain_shape(center + Vector2(4, 12), bench * 1.08, phase), Color("68705e"))
			var rim := _terrain_shape(center + Vector2(0, 5), bench * 1.04, phase)
			draw_colored_polygon(rim, Color("98927a"))
			rim.append(rim[0])
			draw_polyline(rim, Color("626856"), 3.5, true)
			draw_colored_polygon(_terrain_shape(center, bench, phase), Color("b7ac88"))
		draw_colored_polygon(_terrain_shape(center, Vector2(radius, radius * 0.83), phase), Color(0.73, 0.68, 0.49, 0.54))
		draw_colored_polygon(_terrain_shape(center, Vector2(radius * 0.75, radius * 0.60), phase), Color(0.79, 0.73, 0.55, 0.48))
		# Village ground must not paint over real water: repaint raster cells
		# that the disc overlaps so shorelines stay honest at every size.
		if raster_cols > 0 and not raster_classes.is_empty():
			var reach := int(ceil((radius + 10.0) / raster_cell))
			var ccx := int(center.x / raster_cell)
			var ccy := int(center.y / raster_cell)
			for dj in range(-reach, reach + 1):
				for di in range(-reach, reach + 1):
					var rx := ccx + di
					var ry := ccy + dj
					if rx < 0 or ry < 0 or rx >= raster_cols or ry >= raster_rows:
						continue
					var cls := int(raster_classes[ry * raster_cols + rx])
					if cls != 0 and cls != 6:
						continue
					var cell_origin := Vector2(rx, ry) * raster_cell
					if (cell_origin + Vector2.ONE * raster_cell * 0.5).distance_to(center) < radius + raster_cell:
						var water_col := Color("7891c6") if cls == 0 else Color("8fa8d8")
						draw_rect(Rect2(cell_origin - Vector2.ONE, Vector2.ONE * (raster_cell + 2.0)), water_col)

# Raster class at a world position; -1 when no terrain grid is loaded.
func class_at(p: Vector2) -> int:
	if raster_cols <= 0 or raster_classes.is_empty():
		return -1
	var cx := clampi(int(p.x / raster_cell), 0, raster_cols - 1)
	var cy := clampi(int(p.y / raster_cell), 0, raster_rows - 1)
	return int(raster_classes[cy * raster_cols + cx])

# Nudges a point toward `anchor` until it leaves water/rock. Used to keep
# drawn houses on the same dry ground the sim guarantees for beds.
func dry_point(anchor: Vector2, p: Vector2) -> Vector2:
	var bad := [0, 1, 6]
	if raster_classes.is_empty() or not (class_at(p) in bad):
		return p
	for _i in range(24):
		p = p.move_toward(anchor, 16.0)
		if not (class_at(p) in bad):
			return p
	return p

# Keep the painted ruts inside the Rust simulation's 22-unit road corridor.
func _road_shape(start: Vector2, end: Vector2, side: float) -> PackedVector2Array:
	var points := PackedVector2Array()
	var normal := (end - start).normalized().orthogonal() * side
	var bend := minf(12.0, start.distance_to(end) * 0.06)
	for i in range(25):
		var t := i / 24.0
		var drift := bend * sin(PI * t) * (0.47 + 0.53 * sin(TAU * t + 0.5))
		points.append(start.lerp(end, t) + normal * drift)
	return points

func _draw_roads(markets: Array) -> void:
	for road in field.world.get("roads", []):
		var a := int(road.get("a", 0))
		var b := int(road.get("b", 0))
		if a < 0 or b < 0 or a >= markets.size() or b >= markets.size() or a == b:
			continue
		var start := Vector2(markets[a].x, markets[a].y)
		var end := Vector2(markets[b].x, markets[b].y)
		var points := PackedVector2Array()
		var raw: Array = road.get("path", [])
		if raw.size() >= 2:
			for pt in raw:
				points.append(Vector2(pt[0], pt[1]))
		else:
			var side := 1.0 if (mini(a, b) + maxi(a, b)) % 2 == 0 else -1.0
			points = _road_shape(start, end, side)
		if road.built:
			draw_polyline(points, Color(0.33, 0.30, 0.20, 0.26), 18.0, true)
			draw_polyline(points, Color("a7966d"), 12.0, true)
			draw_polyline(points, Color("c7b489"), 7.0, true)
			for i in range(2, points.size() - 1, 2):
				var direction := (points[i + 1] - points[i - 1]).normalized()
				var verge := direction.orthogonal() * (5.0 if i % 4 == 0 else -5.0)
				var at := points[i] + verge
				draw_line(at - direction * 7.0, at + direction * 9.0, Color(0.38, 0.33, 0.20, 0.31), 1.6, true)
		elif field.fidelity > 1 or field.show_trade:
			for i in range(points.size() - 1):
				var first := points[i]
				var last := points[i + 1]
				draw_dashed_line(first, last, Color("687452"), 2.0, 10.0, true)
		var traffic := int(road.get("traffic", 0))
		if field.show_trade and traffic > 0:
			var width: float = clampf(2.0 + log(float(traffic) + 1.0), 3.0, 10.0) / field.zoom
			draw_polyline(points, Color(0.35, 0.90, 0.83, 0.70), width, true)

func _draw_ports() -> void:
	for port in field.world.get("ports", []):
		var home := Vector2(port.x, port.y)
		var landing := Vector2(port.lx, port.ly)
		var near_done: bool = port.get("near_done", false)
		var far_done: bool = port.get("far_done", false)
		draw_dashed_line(home, landing, Color("7d94a8", 0.5), 2.0, 8.0, true)
		_pier(home, landing, near_done)
		_pier(landing, home, far_done)

func _pier(at: Vector2, toward: Vector2, done: bool) -> void:
	# A short dock plank pointing into the water toward its partner.
	var dir := (toward - at).normalized()
	var side := dir.orthogonal()
	var timber := Color("8a6f4d") if done else Color("6b5a45")
	var end := at + dir * 22.0
	draw_line(at, end, timber, 7.0, true)
	for t in [0.45, 0.8, 1.0]:
		var post: Vector2 = at + dir * (22.0 * t)
		draw_line(post - side * 4.5, post + side * 4.5, Color("4d3f2e"), 2.4, true)
	if not done:
		draw_circle(at - dir * 6.0, 4.0, Color("e0b64f", 0.85))

func _draw_ships() -> void:
	for ship in field.world.get("ships", []):
		var pos := Vector2(ship.x, ship.y)
		var ports: Array = field.world.get("ports", [])
		var heading := Vector2(1, 0)
		var pi := int(ship.get("port", -1))
		if pi >= 0 and pi < ports.size():
			var port = ports[pi]
			var dock := Vector2(port.lx, port.ly) if not ship.get("at_far", false) else Vector2(port.x, port.y)
			if pos.distance_to(dock) > 1.0:
				heading = (dock - pos).normalized()
		var side := heading.orthogonal()
		# Flat-sailed river junk: hull, deckhouse and a battened lug sail.
		var hull := PackedVector2Array([pos - heading * 14.0, pos - heading * 8.0 + side * 5.5, pos + heading * 8.0 + side * 5.5, pos + heading * 14.0, pos + heading * 8.0 - side * 5.5, pos - heading * 8.0 - side * 5.5])
		draw_colored_polygon(hull, Color("7a5c38"))
		var closed := PackedVector2Array(hull)
		closed.append(hull[0])
		draw_polyline(closed, Color("4d3a22"), 1.6, true)
		draw_circle(pos + heading * 7.0, 3.2, Color("8a6f4d"))
		var mast := pos - heading * 2.0
		var sail := PackedVector2Array([mast + heading * 1.5, mast - heading * 7.0 + side * 6.5, mast + heading * 9.5 + side * 3.5])
		draw_colored_polygon(sail, Color("d8c9a3") if ship.get("departed", false) else Color("b8ab8e"))
		var riders := int(ship.get("aboard", []).size())
		if riders > 0 and field.fidelity == 1:
			draw_string(ThemeDB.fallback_font, pos + side * 10.0, str(riders), HORIZONTAL_ALIGNMENT_CENTER, -1, 9, Color("2e2618"))
