extends Node2D

@export
var apples = 0:
	set(value):
		var i = 1
		for child in $Contents.get_children():
			if i <= value:
				child.visible = true
			else:
				child.visible = false
			i += 1
		if value >= 6:
			$Label.text = str(value)
			$Label.visible = true
		else:
			$Label.visible = false
