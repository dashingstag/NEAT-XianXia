extends RefCounted

static func describe(info: Dictionary, state: Dictionary) -> String:
	return summary_text(info,state)+mind_text(info,state)+relationships_text(info,state)+policy_text(info,state)

static func summary_text(info: Dictionary, state: Dictionary) -> String:
	var person: Dictionary = info.get("person",{})
	var text := "[color=#e5c78e][font_size=20]INDIVIDUAL MIND / #%s[/font_size][/color]\n" % int(info.id)
	if not bool(info.get("alive",true)):
		text += "[color=#dd9c91][b]DECEASED[/b][/color]\n"
	text += "[b]%s[/b] / %s / age %.1f%s\n" % [person.get("name","Unnamed"),person.get("gender",""),float(person.get("age_years",0))," / child" if person.get("adult",true)==false else ""]
	var home := int(info.get("home",-1))
	var place := "At (%s, %s)" % [roundi(float(info.get("x",0))),roundi(float(info.get("y",0)))]
	var towns: Array = state.get("markets",[])
	if home >= 0 and home < towns.size():
		var away := Vector2(float(info.get("x",0)),float(info.get("y",0))).distance_to(Vector2(float(towns[home].x),float(towns[home].y)))
		place += " · Home: %s%s" % [str(towns[home].name)," · %su away" % roundi(away) if away >= 24.0 else ""]
	text += place + "\n"
	if info.get("action") != null:
		var label := str(info.get("action_label",""))
		text += "Now: [b]%s[/b]%s\n" % [info.action," · %s" % label if not label.is_empty() else ""]
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
	text += "Traits: %s\nAspiration: %s%s\n" % [", ".join(person.get("traits",[])),person.get("aspiration","")," · achieved" if bool(person.get("aspiration_achieved",false)) else ""]
	var spouse: Variant = person.get("spouse")
	text += "Spouse: %s / children %s\n" % [str(spouse.get("name",""))+" #"+str(int(spouse.id)) if spouse is Dictionary else "Unmarried",str(person.get("children",[]))]
	var parents := ""
	for role in ["father","mother"]:
		var kin := int(person.get(role,-1))
		if kin >= 0 and kin < 0xFFFFFF:
			parents += "%s: [url=%s]#%s[/url] · " % [role.capitalize(),kin,kin]
	if not parents.is_empty():
		text += parents.trim_suffix(" · ") + "\n"
	var pregnancy: Variant = person.get("pregnancy")
	if pregnancy is Dictionary:
		text += "Pregnancy: trimester %s / %.1f days remaining%s\n" % [int(pregnancy.trimester),float(pregnancy.days_until_birth)," / settlement required" if pregnancy.settlement_required else ""]
	text += "Health %.0f · energy %.0f · hunger %.0f\nCoins %s · cargo %s %s\n\n" % [info.health,info.energy,info.hunger,info.coins,info.cargo,info.product]
	var leadership_award: Variant = person.get("leadership_award")
	if leadership_award is Dictionary:
		text += "Happiness %.0f · base %.0f · leadership +%s for %.0f more days\n" % [float(person.happiness), float(person.base_happiness), int(leadership_award.amount), maxf(0.0, (float(leadership_award.expires) - float(state.get("tick", 0))) / 3000.0)]
	text += "Rested %.0f/100 / %s / %s\n%s\n" % [float(info.get("rested_meter",0)),"RESTED BUFF" if info.get("rested",false) else "Unrested","SLEEPING" if info.get("sleeping",false) else "Awake",str(info.get("rest_location",""))]
	if info.get("job") is Dictionary:
		var job: Dictionary = info.job
		text += "[color=#e8c080]%s / %s c on completion[/color]\n%s\n" % [job.job,int(job.reward),job.phase]
	text += str(info.get("travel","")) + "\n"
	var purchase = info.get("last_purchase")
	if purchase is Dictionary:
		text += "[url=shop/%s/%s]Shop #%s[/url] / [url=seller/%s/%s]owner at decision #%s[/url] / %s at tick %s / charged %s / shop %s / levy %s\n" % [int(info.id),int(purchase.shop_id),int(purchase.shop_id),int(info.id),int(purchase.owner_id),int(purchase.owner_id),purchase.outcome,purchase.resolution_tick,purchase.buyer_charge,purchase.shop_proceeds,purchase.market_levy]
	return text

static func mind_text(info: Dictionary, _state: Dictionary) -> String:
	var person: Dictionary = info.get("person",{})
	var text := "[color=#a8cea0]SHORT-TERM GOALS[/color]\n"
	if person.get("goals",[]).is_empty():
		text += "No current goals.\n"
	for goal in person.get("goals",[]):
		text += "%s: %s/%s / %.1f days left\n" % [goal.kind,int(goal.progress),int(goal.target),float(goal.days_left)]
	var priorities: Array = person.get("need_priorities", [])
	if not priorities.is_empty():
		text += "\n[color=#a8cea0]NEED PRIORITIES[/color]\n"
		for priority in priorities:
			text += "%s %.0f%% %s\n" % [priority.need, float(priority.urgency) * 100.0, "· recovery overrides other activities" if priority.get("critical", false) else ""]
	text += "\n[color=#a8cea0]EQUIPMENT[/color]\n"
	var tools: Array = person.get("tools",[])
	text += ("%s\n" % ", ".join(tools)) if not tools.is_empty() else "No tools carried.\n"
	var pocket: Dictionary = person.get("carried_food",{})
	var food_bits: Array = []
	for item in ["berries","fish","rice"]:
		if int(pocket.get(item,0)) > 0:
			food_bits.append("%s %s" % [int(pocket[item]),item])
	if int(person.get("carried_meat",0)) > 0:
		food_bits.append("%s meat" % int(person.carried_meat))
	text += "Pocket: %s\n" % (" · ".join(food_bits) if not food_bits.is_empty() else "empty")
	text += "Carrying %s / %s weight · wallet weighs 1 when nonempty\n" % [info.get("carried_weight",0),info.get("capacity",0)]
	return text

static func relationships_text(info: Dictionary, _state: Dictionary) -> String:
	var person: Dictionary = info.get("person",{})
	var text := ""
	var relationships: Array = person.get("relationships",[]).duplicate()
	text += "[color=#a8cea0]RELATIONSHIPS / %s OF 50[/color]\n" % relationships.size()
	if relationships.is_empty():
		return text + "No relationships yet.\n"
	relationships.sort_custom(func(a, b): return int(a.score) > int(b.score))
	text += "[table=6]"
	text += "[cell][b]Name[/b][/cell][cell][b]Bond[/b][/cell][cell][b]Ties[/b][/cell][cell][b]Context[/b][/cell][cell][b]Drift[/b][/cell][cell][b]±[/b][/cell]"
	for relationship in relationships:
		var tone := "#a8cea0" if int(relationship.score)>0 else "#dd9c91"
		var ties: Array = []
		if relationship.get("family",false):
			ties.append("family")
		if relationship.get("shared_trait",false):
			ties.append("trait")
		text += "[cell][url=%s]%s[/url][/cell]" % [int(relationship.id),relationship.name]
		text += "[cell][color=%s]%+d[/color][/cell]" % [tone,int(relationship.score)]
		text += "[cell]%s[/cell]" % ("·".join(ties) if not ties.is_empty() else "—")
		text += "[cell]%s[/cell]" % ", ".join(relationship.get("contexts",[]))
		text += "[cell]%.0fd[/cell]" % float(relationship.days_to_drift)
		text += "[cell]%s%%/%s%%[/cell]" % [int(relationship.positive_chance_percent),int(relationship.negative_chance_percent)]
	text += "[/table]\n"
	return text

static func policy_text(info: Dictionary, _state: Dictionary) -> String:
	var text := ""
	var decision = info.get("last_decision")
	if decision is Dictionary:
		text += "\nLast decision at tick %s: %s (target %s)\n" % [decision.decision_tick,decision.action,decision.target]
		if decision.get("outcome") != null and decision.action in ["HuntWildlife", "EatCarcass", "CraftSpear", "CraftBow", "CollectMeat", "SellMeat", "BuyMeat", "BuyInformation", "SellInformation"]:
			text += "Outcome: %s at tick %s\n" % [str(decision.outcome).replace("_", " "), int(decision.resolution_tick)]
		if decision.get("candidate_counts") is Dictionary:
			for family in decision.candidate_counts:
				var totals = decision.candidate_counts[family]
				if totals[1] > 0:
					text += "At decision: %s admitted %s, omitted %s\n" % [family,totals[0],totals[1]]
	else:
		text += "\nNo decision captured yet.\n"
	for family in info.get("candidate_counts",{}):
		var counts = info.candidate_counts[family]
		if counts[1] > 0:
			text += "%s: %s admitted, %s omitted\n" % [family,counts[0],counts[1]]
	text += "\n[color=#a8cea0]CURRENT POLICY PREVIEW[/color]\n"
	text += "%s nodes / %s genes\n" % [int(info.genome_nodes),int(info.genome_genes)]
	var now := str(info.get("action",""))
	for choice in info.get("choices",[]):
		var current := not now.is_empty() and str(choice.action) == now
		text += "%s%s %s → %.3f%s\n" % ["[color=#e5c78e]▶ " if current else "",choice.action,choice.product,float(choice.score),"[/color]" if current else ""]
	return text
