extends Node2D
# Generated once on the GPU. Shared by every detailed individual.
const COLUMNS := 16
const ROWS := 20
func _draw() -> void:
	for pose in range(ROWS):
		for id in range(COLUMNS):
			draw_set_transform(Vector2((id+0.5)*64.0,(pose+0.5)*64.0),PI/2.0 if pose == 19 else 0.0,Vector2(6.4,5.333333))
			_person(id,pose)
func _person(id: int, pose: int) -> void:
	var p := Vector2.ZERO
	var walk := sin(float(pose-1)*TAU/8.0)*1.1 if pose in range(1,9) else 0.0
	var work := sin(float(pose-9)*TAU/8.0)*0.7 if pose in range(9,17) else 0.0
	var health := 0 if pose == 17 else 100
	var action := 1 if (pose >= 9 and pose <= 16) or pose == 18 else 0
	var tunic := Color.from_hsv(float((id*47)%360)/360.0,0.40,0.62)
	var skin: Color = [Color("cf9c74"),Color("dcb58c"),Color("a97855"),Color("edc5a0")][id%4]
	if health == 0:
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
	if action in [1,4,8]:
		draw_line(p+Vector2(2.2,0.5),p+Vector2(3.5+work,-2.2-work),Color("7a5535"),0.55)
		draw_line(p+Vector2(2.5+work,-2.2-work),p+Vector2(4.5+work,-2.2-work),Color("c0c4b6"),0.7)

func draw_ellipse_shadow(p: Vector2) -> void:
	draw_circle(p+Vector2(0.5,2.2),2.3,Color(0.08,0.13,0.08,0.22))
