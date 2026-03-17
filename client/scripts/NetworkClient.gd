extends Node

signal connected(id: int)
signal disconnected()
signal world_snapshot_received(snapshot: Dictionary)
signal player_disconnected_from_server(id: int)
signal player_spawned(player_data: Dictionary)
signal tile_mined(pos: Vector2i)
signal tile_respawned(pos: Vector2i, tile_type: int)
signal mine_progress(pos: Vector2i, progress: float)
signal mine_cancelled()
signal transfer_approved(world_name: String, instance_id: int, spawn_pos: Vector2)
signal transfer_denied(reason: String)
signal zone_changed(zone_type: int)

const SERVER_HOST := "127.0.0.1"
const SERVER_PORT := 7777

var stream: StreamPeerTCP = StreamPeerTCP.new()
var is_connected := false
var my_id: int = -1
var recv_buffer: PackedByteArray = PackedByteArray()

## ClientPacket 0 = MoveInput, 1 = StartMine, 2 = StopMine, 3 = TransferRequest, 4 = Disconnect

func _ready() -> void:
	print("[net] Connecting to %s:%d..." % [SERVER_HOST, SERVER_PORT])
	stream.connect_to_host(SERVER_HOST, SERVER_PORT)

func _process(_delta: float) -> void:
	stream.poll()
	var status := stream.get_status()

	match status:
		StreamPeerTCP.STATUS_NONE:
			if is_connected:
				is_connected = false
				disconnected.emit()
		StreamPeerTCP.STATUS_CONNECTING:
			pass
		StreamPeerTCP.STATUS_CONNECTED:
			if not is_connected:
				is_connected = true
				print("[net] TCP connected, waiting for Welcome...")
			_read_packets()
		StreamPeerTCP.STATUS_ERROR:
			push_error("[net] Connection error")
			if is_connected:
				is_connected = false
				disconnected.emit()

func _read_packets() -> void:
	while stream.get_available_bytes() > 0:
		var available := stream.get_available_bytes()
		var chunk := stream.get_data(available)
		if chunk[0] != OK:
			return
		recv_buffer.append_array(chunk[1])

	while recv_buffer.size() >= 4:
		var frame_len: int = recv_buffer.decode_u32(0)
		if recv_buffer.size() < 4 + frame_len:
			break
		var payload := recv_buffer.slice(4, 4 + frame_len)
		recv_buffer = recv_buffer.slice(4 + frame_len)
		_handle_server_packet(payload)

func _handle_server_packet(data: PackedByteArray) -> void:
	if data.size() < 4:
		return
	var variant: int = data.decode_u32(0)
	var offset := 4

	match variant:
		0:  ## Welcome { id: u64 }
			if data.size() >= offset + 8:
				my_id = data.decode_u64(offset)
				print("[net] Welcome! My ID = %d" % my_id)
				connected.emit(my_id)

		1:  ## WorldSnapshot { tick, players[] }
			_parse_world_snapshot(data, offset)

		2:  ## PlayerDisconnected { id: u64 }
			if data.size() >= offset + 8:
				var pid: int = data.decode_u64(offset)
				player_disconnected_from_server.emit(pid)

		3:  ## TileMined { pos: TilePos { x: i32, y: i32 } }
			if data.size() >= offset + 8:
				var tx: int = data.decode_s32(offset)
				var ty: int = data.decode_s32(offset + 4)
				tile_mined.emit(Vector2i(tx, ty))

		4:  ## TileRespawned { pos, tile_type }
			if data.size() >= offset + 12:
				var tx: int = data.decode_s32(offset)
				var ty: int = data.decode_s32(offset + 4)
				var tt: int = data.decode_u32(offset + 8)
				tile_respawned.emit(Vector2i(tx, ty), tt)

		5:  ## MineProgress { pos, progress }
			if data.size() >= offset + 12:
				var tx: int = data.decode_s32(offset)
				var ty: int = data.decode_s32(offset + 4)
				var prog: float = data.decode_float(offset + 8)
				mine_progress.emit(Vector2i(tx, ty), prog)

		6:  ## MineCancelled
			mine_cancelled.emit()

		7:  ## TransferApproved { world: WorldId, spawn_x, spawn_y }
			_parse_transfer_approved(data, offset)

		8:  ## TransferDenied { reason: String }
			_parse_transfer_denied(data, offset)

		9:  # ZoneChanged { zone: ZoneType }
			if data.size() >= offset + 4:
				var zt: int = data.decode_u32(offset)
				zone_changed.emit(zt)

		10: # PlayerSpawned(PlayerState)
			if data.size() >= offset + 20:
				var pid: int = data.decode_u64(offset)
				var px: float = data.decode_float(offset + 8)
				var py: float = data.decode_float(offset + 12)
				var last_seq: int = data.decode_u32(offset + 16)
				player_spawned.emit({
					"id": pid, "x": px, "y": py, "last_input_seq": last_seq
				})

func _parse_world_snapshot(data: PackedByteArray, offset: int) -> void:
	if data.size() < offset + 16:
		return
	
	var tick: int = data.decode_u64(offset)
	offset += 8
	
	var count: int = data.decode_u64(offset)
	offset += 8

	var players := []
	
	for i in range(count):
		if data.size() < offset + 20:
			break
		var pid: int = data.decode_u64(offset)
		var px: float = data.decode_float(offset + 8)
		var py: float = data.decode_float(offset + 12)
		var last_seq: int = data.decode_u32(offset + 16)
		offset += 20
		players.append({
			"id": pid, "x": px, "y": py, "last_input_seq": last_seq
		})

	world_snapshot_received.emit({"tick": tick, "players": players})

func _parse_transfer_approved(data: PackedByteArray, offset: int) -> void:
	## WorldId { world_name: String, instance_id: u32 }
	if data.size() < offset + 8:
		return
	
	var name_len: int = data.decode_u64(offset)
	offset += 8
	
	if data.size() < offset + name_len + 4 + 8:
		return
	
	var name_bytes := data.slice(offset, offset + name_len)
	var world_name := name_bytes.get_string_from_utf8()
	
	offset += name_len
	var inst_id: int = data.decode_u32(offset)
	
	offset += 4
	var sx: float = data.decode_float(offset)
	var sy: float = data.decode_float(offset + 4)
	
	transfer_approved.emit(world_name, inst_id, Vector2(sx, sy))

func _parse_transfer_denied(data: PackedByteArray, offset: int) -> void:
	if data.size() < offset + 8:
		return
	
	var reason_len: int = data.decode_u64(offset)
	offset += 8
	
	if data.size() < offset + reason_len:
		return
	
	var reason_bytes := data.slice(offset, offset + reason_len)
	var reason := reason_bytes.get_string_from_utf8()
	transfer_denied.emit(reason)

func send_move_input(sequence: int, dir_x: float, dir_y: float, dt: float) -> void:
	if not is_connected:
		return
	
	var payload := PackedByteArray()
	payload.resize(20)
	payload.encode_u32(0, 0)
	payload.encode_u32(4, sequence)
	payload.encode_float(8, dir_x)
	payload.encode_float(12, dir_y)
	payload.encode_float(16, dt)
	_send_frame(payload)

func send_start_mine(tile_x: int, tile_y: int) -> void:
	if not is_connected:
		return
	
	## ClientPacket::StartMine { pos: TilePos { x: i32, y: i32 } }
	var payload := PackedByteArray()
	payload.resize(12)
	payload.encode_u32(0, 1)  # start mining
	payload.encode_s32(4, tile_x) 
	payload.encode_s32(8, tile_y)
	_send_frame(payload)

func send_stop_mine() -> void:
	if not is_connected:
		return
	
	var payload := PackedByteArray()
	payload.resize(4)
	payload.encode_u32(0, 2)
	_send_frame(payload)

func send_transfer_request(world_name: String, instance_id: int) -> void:
	if not is_connected:
		return
	
	## ClientPacket::TransferRequest { target: WorldId { world_name, instance_id } }
	var name_bytes := world_name.to_utf8_buffer()
	var payload := PackedByteArray()
	
	payload.resize(4 + 8 + name_bytes.size() + 4)
	payload.encode_u32(0, 3)
	payload.encode_u64(4, name_bytes.size())
	
	for i in range(name_bytes.size()):
		payload[12 + i] = name_bytes[i]
	
	payload.encode_u32(12 + name_bytes.size(), instance_id)
	_send_frame(payload)

func send_disconnect() -> void:
	if not is_connected:
		return
	
	var payload := PackedByteArray()
	payload.resize(4)
	payload.encode_u32(0, 4)
	_send_frame(payload)

func _send_frame(payload: PackedByteArray) -> void:
	var header := PackedByteArray()
	header.resize(4)
	header.encode_u32(0, payload.size())
	stream.put_data(header)
	stream.put_data(payload)

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		send_disconnect()
		stream.disconnect_from_host()
