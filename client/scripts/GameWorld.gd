extends Node2D

@onready var network: Node = %NetworkClient
@onready var local_player: Node2D = %PlayerController

var remote_players: Dictionary = {}

const RemotePlayerScene := preload("res://scenes/RemotePlayer.tscn")

func _ready() -> void:
	network.connected.connect(_on_connected)
	network.world_snapshot_received.connect(_on_world_snapshot)
	network.player_disconnected_from_server.connect(_on_player_disconnected)
	network.player_spawned.connect(_on_player_spawned)
	network.tile_mined.connect(_on_tile_mined)
	network.tile_respawned.connect(_on_tile_respawned)
	network.mine_progress.connect(_on_mine_progress)
	network.mine_cancelled.connect(_on_mine_cancelled)
	network.transfer_approved.connect(_on_transfer_approved)
	network.transfer_denied.connect(_on_transfer_denied)

func _on_connected(_id: int) -> void:
	print("[game] Connected!")

func _on_world_snapshot(snapshot: Dictionary) -> void:
	var my_id: int = network.my_id
	
	for player_data: Dictionary in snapshot["players"]:
		var pid: int = player_data["id"]
		if pid == my_id:
			continue
		if not remote_players.has(pid):
			_spawn_remote_player(pid)
		remote_players[pid].update_state(player_data["x"], player_data["y"])

func _on_player_spawned(player_data: Dictionary) -> void:
	var pid: int = player_data["id"]
	if pid == network.my_id:
		return
	if not remote_players.has(pid):
		_spawn_remote_player(pid)
	remote_players[pid].update_state(player_data["x"], player_data["y"])

func _spawn_remote_player(pid: int) -> void:
	var node: Node2D = RemotePlayerScene.instantiate()
	node.player_id = pid
	add_child(node)
	remote_players[pid] = node
	print("[game] Spawned remote player %d" % pid)

func _on_player_disconnected(pid: int) -> void:
	if remote_players.has(pid):
		remote_players[pid].queue_free()
		remote_players.erase(pid)
		print("[game] Removed remote player %d" % pid)

## Tile events (most are todo as this code works on server implementation first)
func _on_tile_mined(pos: Vector2i) -> void:
	## TODO: Remove tile from mining TileMapLayer at pos
	print("[game] Tile mined at %s" % str(pos))

func _on_tile_respawned(pos: Vector2i, tile_type: int) -> void:
	## TODO: Add tile back to mining TileMapLayer at pos
	print("[game] Tile respawned at %s (type %d)" % [str(pos), tile_type])

func _on_mine_progress(pos: Vector2i, progress: float) -> void:
	## TODO: Update mining progress bar UI
	pass

func _on_mine_cancelled() -> void:
	## TODO: Cancel mining animation/UI
	print("[game] Mining cancelled")

## Transfer events
func _on_transfer_approved(world_name: String, instance_id: int, spawn_pos: Vector2) -> void:
	print("[game] Transfer approved to %s:%d at %s" % [world_name, instance_id, str(spawn_pos)])
	local_player.position = spawn_pos
	## TODO: Load new world tilemap, clear remote players
	for pid: int in remote_players:
		remote_players[pid].queue_free()
	remote_players.clear()

func _on_transfer_denied(reason: String) -> void:
	print("[game] Transfer denied: %s" % reason)

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		network.send_disconnect()
