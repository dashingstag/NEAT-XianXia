extends SceneTree
# Uses a saved CLI report only. Never connects to the player's backend.
class OfflineView extends "res://main.gd":
	var commands: Array = []
	func send(command: Dictionary) -> void:
		commands.append(command)
	func _connect() -> void:
		pass
	func _process(_delta: float) -> void:
		pass

var ui: Control
var report: Dictionary
var checked := 0

func _initialize() -> void:
	call_deferred("check_ui")

func check_ui() -> void:
	var args := OS.get_cmdline_user_args()
	assert(args.size() >= 1,"Pass a native urban-demo report path")
	report = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var world: Dictionary = report.world
	assert(not world.urban.zones.is_empty())
	world.selected = report.business_candidates[0] if report.has("business_candidates") else null
	ui = OfflineView.new()
	root.add_child(ui)
	ui.size = Vector2(1440,900)
	await process_frame
	var metadata := JSON.stringify(world).to_utf8_buffer()
	var packet := PackedByteArray()
	packet.resize(32+metadata.size())
	for k in range(4):
		packet[k] = "XNS1".unicode_at(k)
	packet.encode_u16(4,2)
	packet.encode_u16(6,1)
	packet.encode_u32(24,metadata.size())
	packet.encode_u32(28,0)
	for k in range(metadata.size()):
		packet[32+k] = metadata[k]
	ui.last_inspector_ms = -1000
	ui._decode(packet)
	if not world.has("hill"):
		assert(ui.field.hill_footprint().get_center() == Vector2(1500,1380))
	var original_world: Dictionary = ui.field.world
	ui.field.world = original_world.duplicate()
	ui.field.world.hill = {"x":1840,"y":720,"radius":180}
	assert(ui.field.hill_footprint() == Rect2(1660,540,360,360))
	ui.field.world = original_world
	if "--preview" in args:
		ui.status.text = "Offline town fixture"
		return
	assert(ui.counter.text.contains("BALANCED"))
	var overlays := {}
	for control in ui.find_children("*", "Button", true, false):
		if control.text in ["Trade", "Resources"]:
			overlays[control.text] = control
	assert(overlays.size() == 2)
	assert(not ui.field.show_trade and not ui.field.show_resources)
	overlays["Trade"].button_pressed = true
	overlays["Resources"].button_pressed = true
	assert(ui.field.show_trade and ui.field.show_resources)
	for level in [1, 2, 3]:
		ui.field.set_fidelity(level)
		await process_frame
		assert(ui.field.show_trade and ui.field.show_resources)
	overlays["Trade"].button_pressed = false
	overlays["Resources"].button_pressed = false
	assert(not ui.field.show_trade and not ui.field.show_resources)
	assert(ui.field.detail_mesh.texture == ui.field._tex("villager-atlas-v2/atlas.png"))
	assert(ui.field.detail_mesh.texture.get_size() == Vector2(1024,1280))
	assert(ui.field._tex("animal-table-xianxia/atlas.png").get_size() == Vector2(576,384))
	for group in ["terrain-flora-xianxia/pine.png","site-glyphs-xianxia/shop.png","site-glyphs-xianxia/bank.png"]:
		assert(ui.field._tex(group) != null)
	for item in ui.resource_option.item_count:
		assert(ui.resource_option.get_item_icon(item) != null)
	ui.connected = true
	ui.inspector_tabs.current_tab = ui.inspector_tabs.get_tab_idx_from_control(ui.inspector_tabs.get_node("Wealth"))
	assert(ui.commands.back() == {"command":"watch_wealth","enabled":true})
	ui.inspector_tabs.current_tab = 0
	assert(ui.commands.back() == {"command":"watch_wealth","enabled":false})
	ui.connected = false
	assert(ui.markets.text.contains("DISTRICTS / PROPERTY / BUSINESS"))
	assert(ui.markets.text.contains("Town"))
	for job in world.urban.get("operator_jobs",[]):
		assert(ui.markets.text.contains(job.title))
		assert(int(job.required_coins)==int(job.entry_fee)+int(job.working_capital))
	assert(ui.persona_option.selected==int(world.urban.villages[0].persona_id))
	if world.has("leaderboard"):
		var rows: Array = world.leaderboard.rows
		assert(rows.size()<=30 and not rows.is_empty())
		for row in rows:
			assert(ui.leaderboard.text.contains(row.name))
			assert(int(row.net_worth)==int(row.coins)+int(row.assets_value)-int(row.get("debt",row.arrears)))
		ui.leaderboard.meta_clicked.emit(str(int(rows[0].id)))
		assert(int(ui.selected_input.value)==int(rows[0].id))
		assert(ui.inspector_tabs.current_tab==0)
	if world.selected is Dictionary:
		assert(ui.person_tabs.visible and ui.person_tabs.get_tab_count() == 4)
		assert(not ui.detail.visible)
		assert(ui.summary_label.text.contains("INDIVIDUAL MIND"))
		assert(ui.policy_label.text.contains("CURRENT POLICY PREVIEW"))
		var relations: Array = world.selected.person.get("relationships",[])
		assert(relations.size()<=50)
		var rel_rows: Array = ui.relations_tree.get_root().get_children()
		assert(rel_rows.size() == relations.size())
		var names: Array = rel_rows.map(func(row): return row.get_text(0))
		var scores := {}
		var best_score := -999
		for relationship in relations:
			assert(relationship.name in names)
			assert(int(relationship.score)!=0 and absi(int(relationship.score))<=100)
			scores[int(relationship.id)] = int(relationship.score)
			best_score = maxi(best_score,int(relationship.score))
		if not relations.is_empty():
			assert(scores.get(int(rel_rows[0].get_metadata(0)),-999) == best_score)
			rel_rows[0].select(0)
			assert(int(ui.selected_input.value) == int(rel_rows[0].get_metadata(0)))
	# Overlay a synthetic receipt on the legacy fixture; its shop has since changed hands.
	var buyer: Dictionary = world.selected.duplicate(true)
	var original := int(buyer.id)
	for building in world.urban.buildings:
		if int(building.kind) != 1 or building.owner == null:
			continue
		var shop := int(building.id)
		var current := int(building.owner)
		var previous := current + 1
		buyer.last_purchase = {"shop_id":shop,"owner_id":previous,"outcome":"purchased","resolution_tick":120,"buyer_charge":8,"shop_proceeds":7,"market_levy":1}
		var receipt: String = ui.PersonInspector.describe(buyer,world)
		var shop_link := "shop/%s/%s" % [original,shop]
		var seller_link := "seller/%s/%s" % [original,previous]
		assert(receipt.contains("[url=%s]" % shop_link) and receipt.contains("[url=%s]" % seller_link))
		ui.detail.meta_clicked.emit(seller_link)
		assert(int(ui.selected_input.value)==previous)
		ui.detail.meta_clicked.emit("buyer")
		assert(int(ui.selected_input.value)==original)
		ui.detail.meta_clicked.emit(shop_link)
		assert(ui.site_selection.id==shop and ui.detail.text.contains("[url=buyer]"))
		var owner_link := "owner/%s" % current
		assert(ui.detail.text.contains("[url=%s]" % owner_link))
		ui.detail.meta_clicked.emit(owner_link)
		assert(int(ui.selected_input.value)==current and ui.purchase_buyer==original)
		ui.detail.meta_clicked.emit("buyer")
		assert(int(ui.selected_input.value)==original and ui.purchase_buyer==-1)
		break
	assert(buyer.has("last_purchase"),"Expected an occupied shop for navigation")
	ui.field.show_zones = true
	ui.field.pick_mode = "Buildings"
	for building in world.urban.buildings:
		var position := Vector2(building.x,building.y)
		var picked: Dictionary = ui.field._pick_site(position)
		assert(picked.get("kind","")=="urban_buildings")
		assert(int(picked.id)==int(building.id))
		ui.site_selection = picked
		var description: String = ui._site_text()
		assert(description.contains("Status:"))
		assert(description.contains("FINISHED WARES"))
		var resolved: Dictionary = ui._site_data()
		assert(Vector2(resolved.x,resolved.y).distance_to(position)<0.01)
		ui._centre_selection()
		checked += 1
	for bank in world.get("banks",[]):
		var picked: Dictionary = ui.field._pick_site(Vector2(bank.x,bank.y))
		assert(picked.get("kind","")=="banks")
		ui.site_selection=picked
		assert(ui._site_text().contains("Annual lending rate"))
		ui._centre_selection()
	for market in world.get("markets",[]):
		ui.site_selection = {"kind":"markets","id":int(market.id),"market":int(market.id)}
		var description: String = ui._site_text()
		assert(description.contains("GOODS held"))
		for k in range(market.stock.size()):
			var q: Dictionary = market.quotes[k]
			if int(market.stock[k])==0 and int(q.offered)==0 and int(q.remaining)==0:
				continue
			assert(description.contains("%s: %s held" % [world.products[k],int(market.stock[k])]))
			if int(q.offered)>0 or int(q.ask)>0:
				assert(description.contains("%s @ %sc" % [int(q.offered),int(q.ask)]))
			if int(q.remaining)>0 or int(q.bid)>0:
				assert(description.contains("buys %s @ %sc" % [int(q.remaining),int(q.bid)]))
		ui._centre_selection()
	for level in [1,2,3]:
		ui.field.set_fidelity(level)
		ui.field.queue_redraw()
		await process_frame
		await process_frame
	var carcass := {"id": 27, "x": 1200.0, "y": 700.0, "species": "Deer", "portions": 2}
	ui.state.wildlife["carcasses"] = [carcass]
	ui.field.world = ui.state
	ui.field.pick_mode = "Wildlife"
	ui.field.set_fidelity(1)
	var at := Vector2(carcass.x, carcass.y)
	ui.field._select_at(ui.field.offset + at * ui.field.zoom)
	assert(ui.site_selection.kind == "carcasses" and int(ui.site_selection.id) == 27)
	assert(ui._site_text().contains("Deer carcass") and ui._site_text().contains("2 meat portions remaining"))
	carcass.portions = 1
	assert(ui._site_text().contains("1 meat portion remaining"))
	ui.state.wildlife.carcasses.clear()
	assert(ui.field._pick_carcass(at) == -1)
	assert(ui._site_text().contains("depleted"))

	var animal := {"id": 2, "species": "Fox", "gender": "Male", "age_days": 300, "adult": true,
		"health": 100.0, "hunger": 20.0, "energy": 80.0, "forest": 0, "action": "Rest",
		"generation": 0, "genome_nodes": 49, "genome_genes": 48, "x": 100.0, "y": 200.0}
	assert(ui._animal_text(animal).contains("Diet: prey and carcass meat."))
	assert(ui._animal_text(animal).contains("At (100, 200)"))
	animal.species = "Rabbit"
	assert(ui._animal_text(animal).contains("Diet: grass or forest berries."))
	animal.species = "Fox"

	# Vitals bars and status chips mirror the inspected subject.
	ui._update_vitals(animal,"animal")
	assert(ui.vital_rows["health"].bar.value == 100.0)
	assert(ui.vital_rows["hunger"].bar.value == 20.0)
	assert(not ui.vital_rows["rested"].row.visible)
	var animal_chips: Array = ui.chips_row.get_children().map(func(c): return c.text)
	assert("PREDATOR" in animal_chips)

	var probe: Dictionary = world.selected.duplicate(true)
	probe.health = 40.0; probe.hunger = 88.0
	probe.person["adult"] = false
	probe.person["need_priorities"] = [{"need":"food","urgency":0.9,"critical":true}]
	ui._update_vitals(probe,"person")
	assert(ui.vital_rows["health"].bar.value == 40.0)
	assert(ui.vital_rows["hunger"].bar.value == 88.0)
	assert(ui.vital_rows["rested"].row.visible)
	var chips: Array = ui.chips_row.get_children().map(func(c): return c.text)
	assert("STARVING" in chips and "CHILD" in chips)
	assert(ui.needs_box.get_child_count() == 1)

	probe["action"] = "ForageMeal"; probe["action_label"] = "berries"
	probe["x"] = 12.0; probe["y"] = 34.0; probe["home"] = 0
	probe.person["father"] = 7
	probe.person["carried_food"] = {"berries":3,"fish":1,"rice":0}
	var text: String = ui.PersonInspector.describe(probe, world)
	assert(text.contains("At (12, 34)"))
	assert(text.contains("Home: %s" % world.markets[0].name))
	assert(text.contains("Now: [b]ForageMeal[/b] · berries"))
	assert(text.contains("Father: [url=7]#7[/url]"))
	assert(text.contains("Pocket: 3 berries · 1 fish"))
	assert(text.contains("/ child"))
	if not probe.choices.is_empty():
		probe.choices[0].action = "ForageMeal"
		assert(ui.PersonInspector.describe(probe, world).contains("▶ ForageMeal"))
	print(JSON.stringify({"urban_ui":"passed","buildings_inspected":checked,"zones":world.urban.zones.size(),"fidelity_levels":3,"connected_to_live_backend":false}))
	quit()
