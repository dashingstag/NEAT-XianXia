extends RefCounted

static func describe(info: Dictionary, state: Dictionary) -> String:
	var person: Dictionary = info.get("person",{})
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
	var leadership_award: Variant = person.get("leadership_award")
	if leadership_award is Dictionary:
		text += "Happiness %.0f · base %.0f · leadership +%s for %.0f more days\n" % [float(person.happiness), float(person.base_happiness), int(leadership_award.amount), maxf(0.0, (float(leadership_award.expires) - float(state.get("tick", 0))) / 3000.0)]
	text += "Carrying %s / %s weight · wallet weighs 1 when nonempty\n" % [info.get("carried_weight",0),info.get("capacity",0)]
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
	var decision = info.get("last_decision")
	if decision is Dictionary:
		text += "\nLast decision at tick %s: %s (target %s)\n" % [decision.decision_tick,decision.action,decision.target]
		if decision.get("candidate_counts") is Dictionary:
			for family in decision.candidate_counts:
				var totals = decision.candidate_counts[family]
				if totals[1] > 0:
					text += "At decision: %s admitted %s, omitted %s\n" % [family,totals[0],totals[1]]
	else:
		text += "\nNo decision captured yet.\n"
	var purchase = info.get("last_purchase")
	if purchase is Dictionary:
		text += "[url=shop/%s/%s]Shop #%s[/url] / [url=seller/%s/%s]owner at decision #%s[/url] / %s at tick %s / charged %s / shop %s / levy %s\n" % [int(info.id),int(purchase.shop_id),int(purchase.shop_id),int(info.id),int(purchase.owner_id),int(purchase.owner_id),purchase.outcome,purchase.resolution_tick,purchase.buyer_charge,purchase.shop_proceeds,purchase.market_levy]
	for family in info.get("candidate_counts",{}):
		var counts = info.candidate_counts[family]
		if counts[1] > 0:
			text += "%s: %s admitted, %s omitted\n" % [family,counts[0],counts[1]]
	text += "\n[color=#a8cea0]CURRENT POLICY PREVIEW[/color]\n"
	text += "%s nodes / %s genes\n" % [int(info.genome_nodes),int(info.genome_genes)]
	for choice in info.get("choices",[]):
		text += "%s %s → %.3f\n" % [choice.action,choice.product,float(choice.score)]
	return text
