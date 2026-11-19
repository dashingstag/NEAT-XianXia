extends Control
const Field = preload("res://field.gd")
var tcp := StreamPeerTCP.new()
var incoming := PackedByteArray()
var connected := false
var paused := true
var state: Dictionary = {}
var field: Control
var status: Label
var counter: Label
var detail: RichTextLabel
var markets: RichTextLabel
var leaderboard: RichTextLabel
var pause_button: Button
var population: OptionButton
var seed_input: SpinBox
var selected_input: SpinBox
var notice: Label
var last_connect := 0.0
var last_inspector_ms := 0
var last_inspector_id := -1
var clock_label: Label
var rested_bar: ProgressBar
var happiness_bar: ProgressBar
var happiness_label: Label
var rest_label: Label
var board_focus := -1
var site_selection: Dictionary = {}
var follow_button: Button
var inspector_tabs: TabContainer
var vitals: VBoxContainer
var resource_option: OptionButton
var resource_quantity: SpinBox
var persona_option: OptionButton

func label(text: String, font_size: int = 16) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_size_override("font_size",font_size)
	return l

func button(text: String, callback: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.pressed.connect(callback)
	return b

func _ready() -> void:
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for edge in ["margin_left","margin_right","margin_top","margin_bottom"]:
		margin.add_theme_constant_override(edge,20)
	add_child(margin)
	var layout := VBoxContainer.new()
	layout.add_theme_constant_override("separation",12)
	margin.add_child(layout)
	var header := HBoxContainer.new()
	layout.add_child(header)
	var title := label("XIANXIA / NATIVE OBSERVATORY",25)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	status = label("Connecting to local simulation",13)
	header.add_child(status)
	layout.add_child(label("Rust + Rayon  /  Individual compiled NEAT brains  /  Godot instanced viewer",13))
	var controls := HBoxContainer.new()
	controls.add_theme_constant_override("separation",8)
	layout.add_child(controls)
	pause_button = button("Run",func(): send({"command":"pause","value":not paused}))
	controls.add_child(pause_button)
	controls.add_child(button("Step 1s",func(): send({"command":"step","ticks":10})))
	controls.add_child(button("Advance 30s",func(): send({"command":"step","ticks":300})))
	controls.add_child(button("Save",func(): send({"command":"save"})))
	controls.add_child(button("Restore",func(): send({"command":"load"})))
	var speed := OptionButton.new()
	for text in ["1x speed","2x speed","4x speed","8x speed"]:
		speed.add_item(text)
	speed.item_selected.connect(func(index: int): send({"command":"speed","value":pow(2,index)}))
	controls.add_child(speed)
	population = OptionButton.new()
	for number in [1000,10000,100000]:
		population.add_item(str(number)+" agents",number)
	population.select(1)
	controls.add_child(population)
	seed_input = SpinBox.new()
	seed_input.min_value = 1
	seed_input.max_value = 1000000
	seed_input.value = 42
	seed_input.prefix = "Seed "
	controls.add_child(seed_input)
	controls.add_child(button("New world",func(): send({"command":"reset","agents":population.get_selected_id(),"seed":int(seed_input.value)})))
	controls.add_child(button("1 day",func(): send({"command":"advance_days","days":1})))
	controls.add_child(button("15 crop days",func(): send({"command":"advance_days","days":15})))
	counter = label("Waiting for a native snapshot…",15)
	layout.add_child(counter)
	var body := HSplitContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	layout.add_child(body)
	var view_controls := HBoxContainer.new()
	layout.add_child(view_controls)
	layout.move_child(view_controls,layout.get_child_count()-2)
	view_controls.add_child(button("1 · Overview",func(): field.set_fidelity(1)))
	view_controls.add_child(button("2 · Features",func(): field.set_fidelity(2)))
	view_controls.add_child(button("3 · Animation",func(): field.set_fidelity(3)))
	view_controls.add_child(button("Dawn",func(): send({"command":"time_of_day","hour":6.0})))
	view_controls.add_child(button("Night",func(): send({"command":"time_of_day","hour":20.0})))
	view_controls.add_child(button("Wildlife",_show_wildlife))
	var zones_button := button("Zones",func(): pass)
	zones_button.toggle_mode = true
	zones_button.toggled.connect(func(value: bool): field.show_zones=value; field.queue_redraw())
	view_controls.add_child(zones_button)
	clock_label = label("Day 1 / 06:00",12)
	view_controls.add_child(clock_label)
	view_controls.add_child(label("Wheel zoom / left drag pan / click select",12))
	field = Field.new()
	field.agent_selected.connect(_select_agent)
	field.animal_selected.connect(_select_animal)
	field.board_selected.connect(func(market: int): board_focus = market; last_inspector_ms = 0)
	field.site_selected.connect(_select_site)
	field.resource_placed.connect(_place_resource)
	field.follow_cancelled.connect(func(): follow_button.set_pressed_no_signal(false))
	body.add_child(field)
	var panel := VBoxContainer.new()
	panel.custom_minimum_size = Vector2(380,400)
	panel.add_theme_constant_override("separation",10)
	body.add_child(panel)
	panel.add_child(label("OBSERVATORY / INSPECTOR",16))
	var target_row := HBoxContainer.new()
	panel.add_child(target_row)
	target_row.add_child(label("Map selects",12))
	var pick := OptionButton.new()
	for mode in ["Auto","People","Wildlife","Buildings","Resources","Forests"]:
		pick.add_item(mode)
	pick.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pick.item_selected.connect(func(index: int): field.pick_mode=pick.get_item_text(index))
	target_row.add_child(pick)
	inspector_tabs = TabContainer.new()
	inspector_tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	panel.add_child(inspector_tabs)
	var inspect_panel := VBoxContainer.new()
	inspect_panel.name = "Inspect"
	inspect_panel.add_theme_constant_override("separation",8)
	inspector_tabs.add_child(inspect_panel)
	var inspect := HBoxContainer.new()
	inspect_panel.add_child(inspect)
	inspect.add_child(label("Person #",12))
	selected_input = SpinBox.new()
	selected_input.max_value = 1199999
	selected_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inspect.add_child(selected_input)
	inspect.add_child(button("Select",func(): _select_agent(int(selected_input.value))))
	var camera_controls := HBoxContainer.new()
	inspect_panel.add_child(camera_controls)
	camera_controls.add_child(button("Centre",_centre_selection))
	follow_button = button("Follow",func(): pass)
	follow_button.toggle_mode = true
	follow_button.toggled.connect(func(value: bool):
		field.follow_selection=value
		if value:
			_centre_selection()
	)
	camera_controls.add_child(follow_button)
	camera_controls.add_child(label("Drag releases follow",11))
	vitals = VBoxContainer.new()
	inspect_panel.add_child(vitals)
	happiness_label = label("Happiness / selected person",12)
	vitals.add_child(happiness_label)
	happiness_bar = ProgressBar.new()
	happiness_bar.custom_minimum_size = Vector2(200,16)
	happiness_bar.max_value = 100
	vitals.add_child(happiness_bar)
	rest_label = label("Rested meter / selected person",12)
	vitals.add_child(rest_label)
	rested_bar = ProgressBar.new()
	rested_bar.custom_minimum_size = Vector2(200,18)
	rested_bar.max_value = 100
	vitals.add_child(rested_bar)
	detail = RichTextLabel.new()
	detail.bbcode_enabled = true
	detail.meta_clicked.connect(func(meta: Variant): _select_agent(str(meta).to_int()))
	detail.add_theme_font_size_override("normal_font_size",15)
	detail.custom_minimum_size = Vector2(350,200)
	detail.size_flags_vertical = Control.SIZE_EXPAND_FILL
	inspect_panel.add_child(detail)
	var village_panel := VBoxContainer.new()
	village_panel.name = "Village"
	inspector_tabs.add_child(village_panel)
	var board_controls := HBoxContainer.new()
	board_controls.add_child(button("All villages",func(): board_focus = -1; last_inspector_ms = 0))
	board_controls.add_child(button("Test deaths",func(): send({"command":"mortality_test","market":maxi(board_focus,0),"count":5})))
	village_panel.add_child(board_controls)
	var persona_row := HBoxContainer.new()
	persona_row.add_child(label("Council profile",12))
	persona_option = OptionButton.new()
	var persona := persona_option
	for profile in ["Balanced","Forest","Coastal","Rocky"]:
		persona.add_item(profile)
	persona.item_selected.connect(func(index: int): send({"command":"set_persona","market":maxi(board_focus,0),"profile":index}))
	persona_row.add_child(persona)
	village_panel.add_child(persona_row)
	village_panel.add_child(label("Profile applies to focused village (default Willow Reach).",11))
	markets = RichTextLabel.new()
	markets.bbcode_enabled = true
	markets.custom_minimum_size = Vector2(350,200)
	markets.size_flags_vertical = Control.SIZE_EXPAND_FILL
	village_panel.add_child(markets)
	var wealth_panel := VBoxContainer.new()
	wealth_panel.name = "Wealth"
	inspector_tabs.add_child(wealth_panel)
	wealth_panel.add_child(label("WEALTH / TOP 30 INDIVIDUALS",16))
	leaderboard = RichTextLabel.new()
	leaderboard.bbcode_enabled = true
	leaderboard.size_flags_vertical = Control.SIZE_EXPAND_FILL
	leaderboard.add_theme_font_size_override("normal_font_size",14)
	leaderboard.meta_clicked.connect(func(meta: Variant): _select_agent(str(meta).to_int()))
	wealth_panel.add_child(leaderboard)
	var resource_panel := VBoxContainer.new()
	resource_panel.name = "Add resources"
	resource_panel.add_theme_constant_override("separation",12)
	inspector_tabs.add_child(resource_panel)
	resource_panel.add_child(label("Place a resource site",18))
	var help := label("Choose a resource and amount, then click Place and select open ground on the map. Escape cancels. Mines are finite; berries regrow up to their capacity.",13)
	help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	resource_panel.add_child(help)
	resource_option = OptionButton.new()
	for kind in [0,4,5,6,7]:
		resource_option.add_item(["Berries","","","","Gold mine","Silver mine","Copper mine","Iron mine"][kind],kind)
	resource_panel.add_child(resource_option)
	resource_panel.add_child(label("Starting amount / capacity",12))
	resource_quantity = SpinBox.new()
	resource_quantity.min_value = 1
	resource_quantity.max_value = 1000000
	resource_quantity.value = 600
	resource_quantity.step = 100
	resource_panel.add_child(resource_quantity)
	resource_option.item_selected.connect(func(_index: int): resource_quantity.value=600 if resource_option.get_selected_id()==0 else 50000)
	resource_panel.add_child(button("Place on map",func(): field.placement_kind=resource_option.get_selected_id(); notice.text="Click open ground to place %s. Drag still pans; Escape cancels." % resource_option.get_item_text(resource_option.selected)))

	notice = label("Start run-server.ps1 if the native process is not running. No simulation runs in GDScript.",12)
	notice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	layout.add_child(notice)
	_connect()

func _connect() -> void:
	tcp.disconnect_from_host()
	incoming.clear()
	connected = false
	var port := OS.get_environment("XIANXIA_PORT").to_int()
	if port <= 0 or port > 65535:
		status.text = "Start the game with tools/dev.py play"
		last_connect = Time.get_ticks_msec() / 1000.0
		return
	tcp.connect_to_host("127.0.0.1",port)
	last_connect = Time.get_ticks_msec() / 1000.0

func send(command: Dictionary) -> void:
	if tcp.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		notice.text = "Native backend is disconnected. Start run-server.ps1."
		return
	var body := JSON.stringify(command).to_utf8_buffer()
	var frame := PackedByteArray()
	frame.resize(4)
	frame.encode_u32(0,body.size())
	frame.append_array(body)
	tcp.put_data(frame)

func _select_agent(id: int) -> void:
	site_selection.clear()
	field.site_focus = false
	inspector_tabs.current_tab = 0
	selected_input.value = id
	field.selected = id
	field.selected_animal = -1
	last_inspector_ms = 0
	send({"command":"select","agent":id})

func _select_animal(id: int) -> void:
	site_selection.clear()
	field.site_focus = false
	inspector_tabs.current_tab = 0
	field.selected_animal = id
	field.selected = -1
	field.selected_index = -1
	last_inspector_ms = 0
	send({"command":"select_animal","animal":id})

func _show_wildlife() -> void:
	var animals: Array = state.get("wildlife",{}).get("animals",[])
	if animals.is_empty():
		return
	var pick: Dictionary = animals[0]
	for animal in animals:
		if int(animal.id)>field.selected_animal:
			pick = animal
			break
	_select_animal(int(pick.id))
	field.camera_center = Vector2(pick.x,pick.y)
	field.set_fidelity(2)

func _animal_text(animal: Dictionary) -> String:
	var text := "[color=#e5c78e][font_size=20]WILDLIFE MIND / #%s[/font_size][/color]\n" % int(animal.id)
	text += "[b]%s[/b] / %s / %s days old / %s\n" % [animal.species,animal.gender,int(animal.age_days),"Adult" if animal.adult else "Young"]
	text += "Health %.0f / hunger %.0f / food energy %.0f\n" % [float(animal.health),float(animal.hunger),float(animal.energy)]
	text += "Home grove %s / %s\n" % [int(animal.forest),animal.action]
	text += "Mother %s / father %s\n" % [str(animal.mother) if animal.get("mother") != null else "Wild-born",str(animal.father) if animal.get("father") != null else "Wild-born"]
	if animal.get("pregnancy_days_left") != null:
		text += "Pregnancy: %.1f days remaining\n" % float(animal.pregnancy_days_left)
	text += "\nNEAT generation %s / %s nodes / %s genes\n" % [int(animal.generation),int(animal.genome_nodes),int(animal.genome_genes)]
	text += "Diet: grass or forest berries. Grass cannot feed people.\n\n[color=#a8cea0]FEASIBLE ACTION SCORES[/color]\n"
	for choice in animal.get("choices",[]):
		text += "%s: %.3f\n" % [choice.action,float(choice.score)]
	text += "\nSensors: need, benefit, risk, distance, fertility, continuity, species, hunger, energy, health, nearby people, grass, density, night and adulthood."
	return text

func _select_site(site: Dictionary) -> void:
	site_selection = site
	field.site_focus = true
	field.selected = -1
	field.selected_index = -1
	field.selected_animal = -1
	board_focus = int(site.get("market",-1))
	inspector_tabs.current_tab = 0
	last_inspector_ms = 0
	_update_focus_target()
	field.queue_redraw()

func _site_data() -> Dictionary:
	if site_selection.is_empty():
		return {}
	var kind: String = site_selection.kind
	var id := int(site_selection.id)
	if kind=="house":
		var towns: Array = state.get("markets",[])
		var market := int(site_selection.market)
		if market<0 or market>=towns.size() or id>=int(towns[market].housing_blocks):
			return {}
		var town: Dictionary = towns[market]
		var at: Vector2 = (Vector2(town.x,town.y)+field._house_origin(id)).clamp(Vector2(5,5),Vector2(2670,1900))
		return {"x":at.x+10,"y":at.y+7.5,"town":town.name,"capacity":32}
	var rows: Array = state.get("wildlife",{}).get("forests",[]) if kind=="forests" else state.get("urban",{}).get("buildings",[]) if kind=="urban_buildings" else state.get(kind,[])
	if id<0 or id>=rows.size():
		return {}
	var data: Dictionary = rows[id].duplicate()
	if kind=="roads":
		var towns: Array = state.get("markets",[])
		var a: Dictionary = towns[int(data.a)]
		var b: Dictionary = towns[int(data.b)]
		data.x = (float(a.x)+float(b.x))*0.5
		data.y = (float(a.y)+float(b.y))*0.5
		data.route = str(a.name)+" / "+str(b.name)
	return data

func _site_text() -> String:
	var d := _site_data()
	if d.is_empty():
		return "Select a building or resource on the map."
	var kind: String = site_selection.kind
	var title := "Site"
	var body := ""
	match kind:
		"patches":
			var k := int(d.kind)
			var products: Array = state.get("products",[])
			title = str(products[k]).capitalize()+(" mine" if k in [4,5,6,7] else " patch")
			body = "[b]%s remaining[/b] of %s capacity\n" % [int(d.stock),int(d.capacity)]
			if k in [4,5,6,7]:
				body += "[color=#e5c78e]FINITE DEPOSIT[/color]\nNo ore regeneration. The mine disappears when its stock reaches zero.\n"
			else:
				body += "Natural renewal: %.2f units per second, up to capacity.\n" % float(d.renewal)
			body += "\nItem weight: %s per unit\n" % int(state.get("item_weights",[])[k])
			if int(d.stock)==0:
				body += "\n[color=#dda58a]Exhausted[/color]"
		"markets":
			title = str(d.name)
			body = "%s / %s residents\n\n[b]HOUSING[/b]\n%s completed blocks / %s beds\n\n[b]MARKET[/b]\nTreasury: %s copper units\nObserved sales: %s\n" % [d.stage,int(d.population),int(d.housing_blocks),int(d.housing_blocks)*32,int(d.coins),d.profile]
			if d.get("project") is Dictionary:
				body += "\nBuilding %s / %.0f%% complete\n" % [d.project.kind,100.0*float(d.project.progress)/maxf(1.0,float(d.project.required))]
		"house":
			title = "House block %s" % (int(site_selection.id)+1)
			body = "%s residential district\n\n[b]SHELTER[/b]\n32 beds per block\n\n[b]CONSTRUCTION[/b]\n16 timber + 4 stone and completed labour.\nOnly funded, completed houses appear on the map." % d.town
		"schools":
			title = "Village school"
			body = "%s\n\nChildren in village: %s\nClass capacity: %s\nTeacher: %s\nTeacher opening: %s\nBase wage: 10 copper per taught day\n\nChildren aged 5–17 earn Educated after 10 attended school days. Teachers must be 50+ or educated adults.\n\nConstruction: 20 timber + 8 stone and a funded reward.\nEducated workers earn +15%% on village jobs." % ["Open" if d.ready else "Under construction",int(d.children),int(d.capacity),str(int(d.teacher)) if d.get("teacher")!=null else "Unfilled","Posted" if d.teacher_opening else "No unfilled funded posting"]
		"taverns":
			title = "Village tavern"
			body = "%s\n\n[b]SOCIAL SPACE[/b]\n%s / 32 guests\nSocial score grows through time spent with other guests.\n\nConstruction: 12 timber + 6 stone, with a funded reward." % ["Open" if d.ready else "Under construction",int(d.visitors)]
			if not d.ready:
				body += "\nProgress: %.0f%%\n" % (float(d.progress)*5.0)
		"farms":
			title = "Rice farm"
			body = "%s\n\nAge: %.1f days\nTending visits: %s\nRice ready: %s\n\nFertility: %.0f%% / moisture: %.0f%%\n\nCrops require 15 full days and daily tending. Farmer reward: 2 copper per funded shift." % ["Ready to harvest" if d.mature else "Growing",float(d.age_days),int(d.tended_days),int(d.harvest_remaining),float(d.fertility)*100.0,float(d.moisture)*100.0]
		"graveyards":
			title = "Outlying graveyard"
			body = "%s\n\nBurials: %s / %s places\n\nGravediggers carry remains here from the village and surrounding land." % ["Open" if d.ready else "Under construction",int(d.buried_count),int(d.capacity)]
		"job_boards":
			title = str(d.name)+" job board"
			body = "Gravedigger openings: %s\nReward: %s copper\nFarmer openings: %s\n\nTreasury: %s copper\nVillage decision: %s\n\nOpen the Village tab for pressures, rewards and policy details." % [int(d.vacancies),int(d.reward),int(d.get("farmer_openings",0)),int(d.treasury),d.policy]
		"banks":
			title = "Town bank"
			body = "%s / commercial district\nCash reserve %s copper\nAnnual lending rate %s%%\nNext review in %.1f days\nRates move by 1 percentage point/year with trade growth or decline.\n\nActive loans %s\nPrincipal outstanding %s / interest %s\n" % ["Open" if d.ready else "Builder job posted",int(d.cash),int(d.rate_percent),float(d.days_to_review),int(d.active_loans),int(d.principal_outstanding),int(d.interest_outstanding)]
		"urban_buildings":
			title = ["Private home",["Clothes shop","Tool shop","Furniture shop","Butcher shop"][int(d.line)],["Hunter lodge","Cobbler","Blacksmith","Carpenter"][int(d.line)]][int(d.kind)]
			body = "Status: %s\n" % ("Complete" if d.ready else "Requires forest clearance" if d.needs_clear else "Builder contract posted")
			body += "Owner: %s / renter: %s\n" % ["Village" if d.owner==null else "Person #"+str(int(d.owner)),"Vacant" if d.renter==null else "Person #"+str(int(d.renter))]
			body += "Price %s copper / rent %s per day\nArrears %s / business cash %s\n" % [int(d.price),int(d.rent),int(d.arrears),int(d.cash)]
			if int(d.kind)>0:
				body += "Lease entry: 8 fee + 50 working capital\nMarketplace pays 2/day for operating shifts.\nMade %s / retail units sold %s\n" % [int(d.made),int(d.sold)]
			else:
				body += "4 private beds for owner or renter and family.\n"
			body += "\nINPUTS / RAW STOCK\n"
			for k in range(d.raw.size()):
				if int(d.raw[k])>0:
					body += "%s: %s\n" % [state.products[k],int(d.raw[k])]
			body += "\nFINISHED WARES\n"
			for k in range(d.wares.size()):
				if int(d.wares[k])>0:
					body += "%s: %s\n" % [state.urban.ware_names[k],int(d.wares[k])]
		"shelters":
			title = "Campsite"
			body = "Beds reserved: %s / %s\n\nTemporary shelter outside settlements." % [int(d.reserved),int(d.capacity)]
		"roads":
			title = "Trade road"
			body = "%s\n\n%s / %s recorded deliveries\nCompleted roads speed up travel nearby." % [d.route,"Built" if d.built else "Not built",int(d.traffic)]
		"forests":
			title = "Forest grove %s" % (int(site_selection.id)+1)
			var grass_stock := 0
			for grass in state.get("wildlife",{}).get("grass",[]):
				if int(grass.forest)==int(site_selection.id):
					grass_stock += int(grass.stock)
			body = "Woodland radius: %.0f\nGrass available: %s\n\nTrees are scattered within this zone. Rabbits and deer forage on grass and berries here. Grass is inedible to people." % [float(d.radius),grass_stock]
	return "[color=#e5c78e][font_size=23]%s[/font_size][/color]\n[color=#a8b5ac]%s · %s, %s[/color]\n\n%s" % [title,"Building" if kind in ["banks","urban_buildings","house","schools","taverns","markets","graveyards","shelters","job_boards"] else "Landscape",roundi(float(d.x)),roundi(float(d.y)),body]

func _update_focus_target() -> void:
	var target: Variant = _site_data() if not site_selection.is_empty() else state.get("selected_animal",state.get("selected",{}))
	field.has_focus_target = target is Dictionary and target.has("x") and target.has("y")
	if field.has_focus_target:
		field.focus_target = Vector2(target.x,target.y)

func _centre_selection() -> void:
	_update_focus_target()
	if field.has_focus_target:
		field.camera_center = field.focus_target
		field.magnification = maxf(field.magnification,3.0)
		field._resize()

func _place_resource(kind: int, position: Vector2) -> void:
	send({"command":"add_resource","kind":kind,"x":position.x,"y":position.y,"quantity":int(resource_quantity.value)})

func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode==KEY_ESCAPE:
		field.placement_kind = -1
		notice.text = "Placement cancelled."

func _process(_delta: float) -> void:
	tcp.poll()
	if tcp.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		status.text = "Backend disconnected / retrying"
		if Time.get_ticks_msec()/1000.0-last_connect > 2.0:
			_connect()
		return
	if not connected:
		connected = true
		status.text = "CONNECTED / Rust on loopback TCP"
		send({"command":"select","agent":0})
	var available := tcp.get_available_bytes()
	if available > 0:
		var response := tcp.get_data(available)
		if response[0] == OK:
			incoming.append_array(response[1])
	var newest_snapshot := PackedByteArray()
	while incoming.size() >= 4:
		var length := int(incoming.decode_u32(0))
		if length < 4 or length > 4194304:
			notice.text = "Invalid native snapshot frame."
			_connect()
			return
		if incoming.size() < length+4:
			break
		var payload := incoming.slice(4,length+4)
		incoming = incoming.slice(length+4)
		if payload.slice(0,4).get_string_from_ascii() == "XNS1":
			newest_snapshot = payload
		else:
			_decode(payload)
	if not newest_snapshot.is_empty():
		_decode(newest_snapshot)

func _decode(data: PackedByteArray) -> void:
	var magic := data.slice(0,4).get_string_from_ascii()
	if magic == "XNC1":
		var reply: Variant = JSON.parse_string(data.slice(4).get_string_from_utf8())
		if reply is Dictionary:
			notice.text = str(reply.get("message",""))
			if reply.get("ok",false) and notice.text.begins_with("Added resource site #"):
				_select_site({"kind":"patches","id":int(notice.text.get_slice("#",1)),"market":-1})
		return
	if magic != "XNS1" or data.size() < 28 or data.decode_u16(4) != 1:
		return
	paused = data.decode_u16(6) == 1
	var count: int = data.decode_u32(20)
	var json_length: int = data.decode_u32(24)
	if 28+json_length+count*16 != data.size():
		notice.text = "Snapshot size mismatch."
		return
	var decoded: Variant = JSON.parse_string(data.slice(28,28+json_length).get_string_from_utf8())
	if not decoded is Dictionary:
		return
	state = decoded
	state["paused"] = paused
	field.receive(state,data.slice(28+json_length),count)
	_update_focus_target()
	pause_button.text = "Run" if paused else "Pause"
	var inspected: int = int(state.get("selected",{}).get("id",-1)) if state.get("selected") is Dictionary else -1
	var now_ms := Time.get_ticks_msec()
	if now_ms-last_inspector_ms < 250 and inspected == last_inspector_id:
		return
	last_inspector_ms = now_ms
	last_inspector_id = inspected
	var cycle: Dictionary = state.get("clock",{})
	var hour: float = float(cycle.get("hour",6.0))
	var clock_minutes := roundi(hour*60.0)%1440
	clock_label.text = "Day %s / %02d:%02d / %s / travel %s%%" % [int(cycle.get("day",1)),int(clock_minutes/60),clock_minutes%60,"NIGHT" if cycle.get("night",false) else "DAY",roundi(float(cycle.get("speed_multiplier",1))*100)]
	var stats: Dictionary = state.get("stats",{})
	var balanced := true
	for account in state.get("accounts",[]):
		balanced = balanced and int(account.error) == 0
	for account in state.get("urban",{}).get("ware_accounts",[]):
		balanced = balanced and int(account.error)==0
	balanced = balanced and int(state.coins.initial) + int(state.coins.get("minted",0)) == int(state.coins.current)
	counter.text = "%s living / %s brains | %.1fs | %s decisions | %s sales | %s shipments | ledger %s" % [state.living_agents,state.allocated_agents,float(state.seconds),stats.get("decisions",0),stats.get("trades",0),stats.get("shipments",0),"BALANCED" if balanced else "ERROR"]
	counter.text += " | %s convoys" % int(state.get("convoy_count",0))
	var wealth: Dictionary = state.get("leaderboard",{})
	var wealth_text := "[color=#a8cea0]Living people · estimated copper value[/color]\nClick a name to inspect.\n\n"
	for row in wealth.get("rows",[]):
		wealth_text += "[b]%s. [url=%s]%s[/url][/b]\n[color=#e5c78e]Net worth %s copper[/color] / %s\nCoins %s / assets %s / debt %s\nWallet %s · business cash %s · stored cash %s\nHomes %s (%s) · business stock %s\nBelongings / deliveries %s · businesses %s\n\n" % [int(row.rank),int(row.id),row.name,int(row.net_worth),row.village,int(row.coins),int(row.assets_value),int(row.get("debt",row.arrears)),int(row.wallet),int(row.business_cash),int(row.stored_cash),int(row.homes),int(row.homes_value),int(row.business_stock),int(row.belongings),int(row.businesses)]
	wealth_text += "[color=#a8b5ac]%s[/color]" % wealth.get("valuation","Waiting for wealth data")
	leaderboard.text = wealth_text
	var info: Variant = state.get("selected")
	var animal: Variant = state.get("selected_animal")
	vitals.visible = site_selection.is_empty()
	if not site_selection.is_empty():
		detail.text = _site_text()
	elif animal is Dictionary:
		happiness_label.text = "Health / selected animal"
		rest_label.text = "Food energy / selected animal"
		happiness_bar.value = float(animal.health)
		rested_bar.value = float(animal.energy)
		detail.text = _animal_text(animal)
	elif info is Dictionary:
		happiness_label.text = "Happiness / selected person"
		rest_label.text = "Rested meter / selected person"
		var person: Dictionary = info.get("person",{})
		happiness_bar.value = float(person.get("happiness",50))
		var text := "[color=#e5c78e][font_size=20]INDIVIDUAL MIND / #%s[/font_size][/color]\n" % int(info.id)
		text += "[b]%s[/b] / %s / age %.1f\n" % [person.get("name","Unnamed"),person.get("gender",""),float(person.get("age_years",0))]
		text += "Social score: %.0f/100 / optional benefit\n" % float(person.get("social_score",0))
		text += "Schooling: %s/10 days / %s\n" % [int(person.get("school_days",0)),"Qualified to teach" if person.get("teacher_eligible",false) else "Not yet qualified to teach"]
		if "Educated" in person.get("traits",[]):
			text += "Village jobs: +15% coin entitlement\n"
			text += "Education bonus balance: %.2f copper\n" % float(person.get("education_bonus_pending",0))
		for building in state.get("urban",{}).get("buildings",[]):
			if building.owner==int(info.id) or building.renter==int(info.id):
				text += "%s #%s / %s copper per day\n" % [["Home","Shop","Workshop"][int(building.kind)],int(building.id),int(building.rent)]
		for loan in state.get("banking",{}).get("loans",[]):
			if int(loan.person)==int(info.id):
				text += "Business loan: %s principal + %s interest / %s%% annual\nRate fixed for %.1f more days%s\n" % [int(loan.principal),int(loan.interest),int(loan.annual_rate_percent),float(loan.days_to_reprice)," / repayment overdue" if loan.overdue else ""]
		text += "Traits: %s\nAspiration: %s\n" % [", ".join(person.get("traits",[])),person.get("aspiration","")]
		var spouse: Variant = person.get("spouse")
		text += "Spouse: %s / children %s\n" % [str(spouse.get("name",""))+" #"+str(int(spouse.id)) if spouse is Dictionary else "Unmarried",str(person.get("children",[]))]
		var pregnancy: Variant = person.get("pregnancy")
		if pregnancy is Dictionary:
			text += "Pregnancy: trimester %s / %.1f days remaining%s\n" % [int(pregnancy.trimester),float(pregnancy.days_until_birth)," / settlement required" if pregnancy.settlement_required else ""]
		text += "[color=#a8cea0]SHORT-TERM GOALS[/color]\n"
		for goal in person.get("goals",[]):
			text += "%s: %s/%s / %.1f days left\n" % [goal.kind,int(goal.progress),int(goal.target),float(goal.days_left)]
		text += "\nEquipment: %s\n" % str(person.get("tools",[]))
		text += "Health %.0f · energy %.0f · hunger %.0f\nCoins %s · cargo %s %s\n\n" % [info.health,info.energy,info.hunger,info.coins,info.cargo,info.product]
		text += "Carrying %s / %s weight · wallet weighs 1 when nonempty\n" % [info.get("carried_weight",0),info.get("capacity",0)]
		rested_bar.value = float(info.get("rested_meter",0.0))
		text += "Rested %.0f/100 / %s / %s\n%s\n" % [float(info.get("rested_meter",0)),"RESTED BUFF" if info.get("rested",false) else "Unrested","SLEEPING" if info.get("sleeping",false) else "Awake",str(info.get("rest_location",""))]
		if info.get("job") is Dictionary:
			var job: Dictionary = info.job
			text += "[color=#e8c080]%s / %s c on completion[/color]\n%s\n" % [job.job,int(job.reward),job.phase]
		text += str(info.get("travel","")) + "\n\n"
		var relationships: Array = person.get("relationships",[])
		text += "[color=#a8cea0]RELATIONSHIPS / %s OF 50[/color]\n" % relationships.size()
		if relationships.is_empty():
			text += "No relationships yet.\n"
		for relationship in relationships:
			var tone := "#a8cea0" if int(relationship.score)>0 else "#dd9c91"
			text += "[url=%s]%s[/url] [color=%s]%+d[/color]%s%s\n%s / drift check in %.1f days\n" % [int(relationship.id),relationship.name,tone,int(relationship.score)," / family" if relationship.family else ""," / shared trait" if relationship.shared_trait else "",", ".join(relationship.contexts),float(relationship.days_to_drift)]
			text += "Roll +1: %s%% / -1: %s%%\n" % [int(relationship.positive_chance_percent),int(relationship.negative_chance_percent)]
		text += "\n[color=#a8cea0]NEAT / FEASIBLE CHOICES[/color]\n"
		text += "%s nodes / %s genes\n" % [int(info.genome_nodes),int(info.genome_genes)]
		for choice in info.get("choices",[]):
			text += "%s %s → %.3f\n" % [choice.action,choice.product,float(choice.score)]
		detail.text = text
	var ecology: Dictionary = state.get("wildlife",{})
	var grass: Dictionary = ecology.get("grass_account",{})
	var lines := "[color=#a8cea0]WOODLAND WILDLIFE[/color]\nRabbits %s / deer %s / births %s / deaths %s\nGrass %s / eaten %s / berries eaten %s\n\n" % [int(ecology.get("rabbits",0)),int(ecology.get("deer",0)),int(ecology.get("births",0)),int(ecology.get("deaths",0)),int(grass.get("stock",0)),int(grass.get("eaten",0)),int(ecology.get("berries_eaten",0))]
	lines += "[color=#e5c78e]VILLAGE JOB BOARDS[/color]\nClick a village board to inspect its policy.\n"
	for board in state.get("job_boards",[]):
		if board_focus >= 0 and int(board.market) != board_focus:
			continue
		lines += "\n[b]%s[/b] / %s nearby bodies / %s remote\n" % [board.name,int(board.nearby_bodies),int(board.remote_bodies)]
		lines += "Gravedigger: %s openings / %s c each / %s working\n" % [int(board.vacancies),int(board.reward),int(board.active_haulers)]
		lines += "Rice farming: %s paid openings / 2 c per shift / %s plots\n" % [int(board.get("farmer_openings",0)),int(board.get("farm_count",0))]
		lines += "Build graveyard: %s / %s working\n" % ["8 c posted" if board.builder_open else "No opening",int(board.active_builders)]
		lines += "Treasury %s / escrow %s / income %.2f c/s / expense %.2f c/s\n" % [int(board.treasury),int(board.escrow)+int(board.builder_reward)+int(board.get("farmer_escrow",0)),float(board.income_per_second),float(board.expenses_per_second)]
		lines += "Village NEAT: %s nodes / %s genes / %s\n" % [int(board.genome_nodes),int(board.genome_genes),board.policy]
		if str(board.blocked) != "":
			lines += str(board.blocked)+"\n"
		if board_focus >= 0:
			for choice in board.get("policy_choices",[]):
				lines += "%s: %.3f\n" % [choice.action,float(choice.score)]
	var urban: Dictionary = state.get("urban",{})
	for village_profile in urban.get("villages",[]):
		if int(village_profile.market)==maxi(board_focus,0):
			persona_option.select(int(village_profile.persona_id))
	lines += "\n[color=#e5c78e]DISTRICTS / PROPERTY / BUSINESS[/color]\nGazette at 500 copper. Lease review below 100.\nTown requires 3 completed homes, shops and workshops each.\nMarketplace remains the centre.\n"
	for village in urban.get("villages",[]):
		if board_focus>=0 and int(village.market)!=board_focus:
			continue
		lines += "\n[b]%s[/b] / %s / %s profile\nHomes %s/3 / shops %s/3 / workshops %s/3\n%s / %s business vacancies\n" % [state.markets[int(village.market)].name,"Town" if village.town else "Village",village.persona,int(village.completed[0]),int(village.completed[1]),int(village.completed[2]),"Zoned" if village.zoned else "Not gazetted",int(village.lease_openings)]
		lines += "Coffers %s / free cash %s copper\n" % [int(village.get("coffers",0)),int(village.get("free_cash",0))]
		lines += "Vacant: %s commercial / %s industrial\nPosted operator jobs: %s\n" % [int(village.get("vacant_commercial",0)),int(village.get("vacant_industrial",0)),int(village.lease_openings)]
	lines += "\n[color=#a8cea0]BUSINESS OPERATOR JOBS[/color]\n"
	for bank in state.get("banks",[]):
		if board_focus>=0 and int(bank.market)!=board_focus:
			continue
		lines += "\n[b]Town bank[/b] / %s / reserve %s copper\nAnnual rate %s%% / review in %.1f days\n%s active loans / %s principal outstanding\n" % ["Open" if bank.ready else "Builder job: %s copper" % int(bank.builder_reward),int(bank.cash),int(bank.rate_percent),float(bank.days_to_review),int(bank.active_loans),int(bank.principal_outstanding)]
	for job in urban.get("operator_jobs",[]):
		if board_focus>=0 and int(job.market)!=board_focus:
			continue
		lines += "[b]%s[/b] / building #%s / %s\nEntry %s + working capital %s / rent %s copper/day\nSales income%s\n" % [job.title,int(job.building),job.zone,int(job.entry_fee),int(job.working_capital),int(job.rent_per_day)," + up to %s copper/day marketplace shift pay when funded" % int(job.marketplace_pay_per_day) if int(job.marketplace_pay_per_day)>0 else " from workshop wholesale"]
	for job in urban.get("jobs",[]):
		if board_focus>=0 and int(job.market)!=board_focus:
			continue
		lines += "%s / %s c / %s / %s\n" % [job.kind,int(job.reward),"Private" if job.private else "Village", "Open" if job.worker==null else "Worker #"+str(int(job.worker))]
	lines += "Completed %s / evictions %s\n" % [int(urban.get("completed_jobs",0)),int(urban.get("evictions",0))]
	for school in state.get("schools",[]):
		if board_focus>=0 and int(school.market)!=board_focus:
			continue
		lines += "School / %s / %s children\nTeacher: %s / wage 10 copper per taught day\nQualified: age 50+ or educated adult\n" % ["Open" if school.ready else "Building",int(school.children),"Funded opening" if school.teacher_opening else ("At school" if school.get("teacher")!=null else "Not currently posted")]
	for tavern in state.get("taverns",[]):
		if board_focus >= 0 and int(tavern.market) != board_focus:
			continue
		lines += "Tavern / %s / %s of 32 social guests\n" % ["Open" if tavern.ready else "Under construction",int(tavern.visitors)]
	for farm in state.get("farms",[]):
		if board_focus >= 0 and int(farm.market) != board_focus:
			continue
		lines += "Rice plot %s / %.1f days old / %s tending shifts / %s ready\n" % [int(farm.id),float(farm.age_days),int(farm.tended_days),int(farm.harvest_remaining)]
	var population_life: Dictionary = state.get("population_life",{})
	lines += "\nFamilies: %s married people / %s children / %s pregnancies\nBirths %s / marriages %s / goals %s\nTools made %s / rice harvested %s\n" % [int(population_life.get("married_people",0)),int(population_life.get("children",0)),int(population_life.get("pregnancies",0)),int(stats.get("births",0)),int(stats.get("marriages",0)),int(stats.get("goals_completed",0)),int(stats.get("tools_crafted",0)),int(stats.get("rice_harvested",0))]
	lines += "\nBurials %s / completed contracts %s\n\n[color=#e5c78e]CURRENCY / EXCHANGE[/color]" % [int(stats.get("burials",0)),int(stats.get("jobs_completed",0))]
	lines += "\nMinted %s · barters %s\nPaid jobs %s · food jobs %s\n" % [state.coins.get("minted",0),stats.get("barter_trades",0),stats.get("paid_services",0),stats.get("food_services",0)]
	lines += "Sleeping %s / rested %s\n" % [int(cycle.get("sleeping_people",0)),int(cycle.get("rested_people",0))]
	for shelter in state.get("shelters",[]):
		if not shelter.get("camp",false) and int(shelter.get("capacity",0)) > 0:
			lines += "Settlement %s: %s/%s beds reserved\n" % [int(shelter.settlement),int(shelter.reserved),int(shelter.capacity)]
	lines += "Weights: berry 1 · timber 4 · stone 3 · fish 2\nOre: gold 5 · silver 4 · copper 3\nCopper value: gold 100 · silver 10 · copper 1\n\n[color=#e5c78e]REALIZED SALES / OBSERVED PROFILES[/color]\n"
	for market in state.get("markets",[]):
		lines += "\n[b]%s[/b] · %s\n%s\nStock %s\nLocal %s / imported %s\n" % [market.name,market.stage,market.profile,str(market.stock),str(market.local_sales),str(market.imported_sales)]
	markets.text = lines

