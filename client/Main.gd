extends Node2D

@onready var network: Node = $NetworkClient

func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_W:
				network.send_move(0.0, -1.0)
			KEY_S:
				network.send_move(0.0, 1.0)
			KEY_A:
				network.send_move(-1.0, 0.0)
			KEY_D:
				network.send_move(1.0, 0.0)
			KEY_Q:
				network.send_disconnect()
				get_tree().quit()
