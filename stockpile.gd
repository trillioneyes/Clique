extends Node2D

@export
var apples = 0:
	set(value):
		var i = 1
		for child in get_children():
			if i <= value:
				child.visible = true
			else:
				child.visible = false
			i += 1
