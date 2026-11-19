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
	packet.resize(28+metadata.size())
	for k in range(4):
		packet[k] = "XNS1".unicode_at(k)
	packet.encode_u16(4,1)
	packet.encode_u16(6,1)
	packet.encode_u32(24,metadata.size())
	for k in range(metadata.size()):
		packet[28+k] = metadata[k]
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
		var relations: Array = world.selected.person.get("relationships",[])
		assert(relations.size()<=50)
		for relationship in relations:
			assert(ui.detail.text.contains(relationship.name))
			assert(int(relationship.score)!=0 and absi(int(relationship.score))<=100)
		if not relations.is_empty():
			ui.detail.meta_clicked.emit(str(int(relations[0].id)))
			assert(int(ui.selected_input.value)==int(relations[0].id))
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
	for level in [1,2,3]:
		ui.field.set_fidelity(level)
		ui.field.queue_redraw()
		await process_frame
		await process_frame
	print(JSON.stringify({"urban_ui":"passed","buildings_inspected":checked,"zones":world.urban.zones.size(),"fidelity_levels":3,"connected_to_live_backend":false}))
	quit()
