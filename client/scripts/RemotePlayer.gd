extends Node2D

## Interpolation buffer for smooth rendering of remote players
const INTERPOLATION_DELAY := 0.1  ## 100ms behind (2 server ticks at 20Hz)

var player_id: int = -1
var state_buffer: Array[Dictionary] = []
## Each entry: { "time": float, "x": float, "y": float }

var render_time: float = 0.0
var time_offset: float = 0.0

func _ready() -> void:
	## Label to show player ID
	if has_node("Label"):
		$Label.text = "P%d" % player_id

func update_state(x: float, y: float) -> void:
	var now := Time.get_ticks_msec() / 1000.0
	state_buffer.append({"time": now, "x": x, "y": y})

	## Keep only last 1 second of states
	while state_buffer.size() > 2 and state_buffer[0]["time"] < now - 1.0:
		state_buffer.remove_at(0)

func _process(_delta: float) -> void:
	if state_buffer.size() < 2:
		if state_buffer.size() == 1:
			position = Vector2(state_buffer[0]["x"], state_buffer[0]["y"])
		return

	## Render at (now - INTERPOLATION_DELAY) for smoothness
	var now := Time.get_ticks_msec() / 1000.0
	var render_at := now - INTERPOLATION_DELAY

	## Find the two states to interpolate between
	var prev: Dictionary = state_buffer[0]
	var next: Dictionary = state_buffer[1]

	for i in range(state_buffer.size() - 1):
		if state_buffer[i + 1]["time"] >= render_at:
			prev = state_buffer[i]
			next = state_buffer[i + 1]
			break

	var time_span: float = next["time"] - prev["time"]
	if time_span <= 0.0:
		position = Vector2(next["x"], next["y"])
		return

	var t: float = clampf((render_at - prev["time"]) / time_span, 0.0, 1.0)
	position = Vector2(
		lerpf(prev["x"], next["x"], t),
		lerpf(prev["y"], next["y"], t)
	)
