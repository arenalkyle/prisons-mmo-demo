extends Node2D

## Movement speed — MUST match shared_protocol::movement::MOVE_SPEED
const MOVE_SPEED := 200.0
const WORLD_MIN := -2000.0
const WORLD_MAX := 2000.0

@onready var network: Node = $"../NetworkClient"
@onready var sprite: Sprite2D = $Sprite2D  ## a simple colored square

var my_id: int = -1
var input_sequence: int = 0

## Pending inputs that the server hasn't acknowledged yet
var pending_inputs: Array[Dictionary] = []

func _ready() -> void:
	network.connected.connect(_on_connected)
	network.world_snapshot_received.connect(_on_world_snapshot)
	visible = false

func _on_connected(id: int) -> void:
	my_id = id
	visible = true
	position = Vector2.ZERO

func _physics_process(delta: float) -> void:
	if my_id < 0:
		return

	var dir := Vector2.ZERO
	if Input.is_action_pressed("ui_up") or Input.is_key_pressed(KEY_W):
		dir.y -= 1.0
	if Input.is_action_pressed("ui_down") or Input.is_key_pressed(KEY_S):
		dir.y += 1.0
	if Input.is_action_pressed("ui_left") or Input.is_key_pressed(KEY_A):
		dir.x -= 1.0
	if Input.is_action_pressed("ui_right") or Input.is_key_pressed(KEY_D):
		dir.x += 1.0

	# Normalize diagonal
	if dir.length() > 0.0:
		dir = dir.normalized()
	
	## Only send movement packets when the player is actually moving to 
	## minimize network traffic.
	if dir.length() > 0.001:
		input_sequence += 1
		var seq := input_sequence

		## Send to server
		network.send_move_input(seq, dir.x, dir.y, delta)

		## Client-side prediction: apply immediately
		position = _apply_movement(position, dir.x, dir.y, delta)

		## Reconcile
		pending_inputs.append({
			"sequence": seq,
			"dir_x": dir.x,
			"dir_y": dir.y,
			"dt": delta
		})

func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_T:
				print("[game] Requesting transfer to kremwood_1...")
				network.send_transfer_request("kremwood_1", 1)
			KEY_H:
				print("[game] Requesting transfer to cells_hub...")
				network.send_transfer_request("cells_hub", 1)

func _on_world_snapshot(snapshot: Dictionary) -> void:
	for player_data: Dictionary in snapshot["players"]:
		if player_data["id"] == my_id:
			_reconcile(player_data)
			break

func _reconcile(server_state: Dictionary) -> void:
	var server_seq: int = server_state["last_input_seq"]
	var server_pos := Vector2(server_state["x"], server_state["y"])

	## Drop all inputs the server has already processed
	pending_inputs = pending_inputs.filter(
		func(inp: Dictionary) -> bool: return inp["sequence"] > server_seq
	)

	## Start from server's authoritative position and replay unconfirmed inputs
	var reconciled := server_pos
	for inp: Dictionary in pending_inputs:
		reconciled = _apply_movement(reconciled, inp["dir_x"], inp["dir_y"], inp["dt"])

	position = reconciled

## Must match the server's apply_movement exactly
func _apply_movement(pos: Vector2, dir_x: float, dir_y: float, dt: float) -> Vector2:
	dt = clampf(dt, 0.0, 0.1)  # MAX_DT = 0.1
	
	var dx := dir_x
	var dy := dir_y
	var len_sq := dx * dx + dy * dy
	
	if len_sq > 1.0001:
		var length := sqrt(len_sq)
		dx /= length
		dy /= length

	var new_x := clampf(pos.x + dx * MOVE_SPEED * dt, WORLD_MIN, WORLD_MAX)
	var new_y := clampf(pos.y + dy * MOVE_SPEED * dt, WORLD_MIN, WORLD_MAX)
	return Vector2(new_x, new_y)
