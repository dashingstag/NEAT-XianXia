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
# Shore wave accents, bucketed by 256-unit regions so only visible strokes draw.
var shore_buckets := {}

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
		_build_shore()
		map_key = key
		requested_key = ""
	var explored: Dictionary = layers.get("explored", {})
	if key == map_key and not explored.is_empty():
		fog_texture = ImageTexture.create_from_image(fog_image(explored))
		fog_rect = Rect2(0, 0, int(explored.cols)*float(explored.chunk), int(explored.rows)*float(explored.chunk))
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

const TERRAIN_COLORS := [Color("35606d"), Color("9a9682"), Color("71866a"), Color("62804f"), Color("a5ad6c"), Color("93a575")]

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
			if kind == 0:
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

func _draw_shore(view: Rect2) -> void:
	var first := Vector2i(view.position / 256.0) - Vector2i.ONE
	var last := Vector2i(view.end / 256.0) + Vector2i.ONE
	for bx in range(first.x, last.x + 1):
		for by in range(first.y, last.y + 1):
			var lines: PackedVector2Array = shore_buckets.get(Vector2i(bx, by), PackedVector2Array())
			for s in range(0, lines.size(), 2):
				draw_line(lines[s], lines[s + 1], Color(0.76, 0.88, 0.84, 0.38), 1.6, true)

func _glyph(rel_path: String, at: Vector2, w: float, h: float) -> bool:
	var t: Texture2D = field._tex(rel_path)
	if t == null:
		return false
	draw_texture_rect(t, Rect2(at - Vector2(w,h)*0.5, Vector2(w,h)), false)
	return true

func _draw() -> void:
	ground.queue_redraw()
	fog.queue_redraw()
	draw_set_transform(field.offset,0,Vector2(field.zoom,field.zoom))
	var forest_view := Rect2(-field.offset / field.zoom, field.size / field.zoom)
	if field._classic_terrain():
		_draw_coast_art(forest_view)
		_draw_hill_art(forest_view)
	elif field.fidelity == 1:
		_draw_shore(forest_view)
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
			var tree_glyph: String = "terrain-flora-xianxia/"+field.TREE_GLYPHS[(id*3+tree)%field.TREE_GLYPHS.size()]+".png"
			if field.fidelity > 1 and _glyph(tree_glyph,at,22.0+float(tree%4)*2.0,22.0+float(tree%4)*2.0):
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
		draw_circle(Vector2(grass.x,grass.y),7.0,Color("78a65a") if fraction>0.25 else Color("998761"))
	_draw_settlement_ground(markets, forest_view)
	_draw_roads(markets)

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
		draw_line(Vector2(x, y), Vector2(x + 30, y + 6), Color(0.76, 0.88, 0.84, 0.19), 2.0, true)

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
		var side := 1.0 if (mini(a, b) + maxi(a, b)) % 2 == 0 else -1.0
		var points := _road_shape(start, end, side)
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
