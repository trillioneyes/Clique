extends Node2D

func work():
	$"Work Gear".visible = true
	$AnimationPlayer.play("gear")

func rest():
	$"Work Gear".visible = false
