use crate::movement::{MovementInput, MOVE_SPEED, MAX_DT, WORLD_MIN_X, WORLD_MAX_X, WORLD_MIN_Y, WORLD_MAX_Y};

/// Apply a movement input to a position.
/// Ensure parity between client-side prediction and server-side authority
pub fn apply_movement(x: f32, y: f32, input: &MovementInput) -> (f32, f32) {
    let dt = input.dt.min(MAX_DT).max(0.0);

    // Prevent diagonal speed being higher
    let (dx, dy) = normalize_direction(input.dir_x, input.dir_y);

    let new_x = (x + dx * MOVE_SPEED * dt).clamp(WORLD_MIN_X, WORLD_MAX_X);
    let new_y = (y + dy * MOVE_SPEED * dt).clamp(WORLD_MIN_Y, WORLD_MAX_Y);

    (new_x, new_y)
}

fn normalize_direction(x: f32, y: f32) -> (f32, f32) {
    let len_sq = x * x + y * y;
    
    if len_sq <= 0.0001 {
        (0.0, 0.0)
    } else if len_sq > 1.0001 {
        let len = len_sq.sqrt();
        (x / len, y / len)
    } else {
        (x, y)
    }
}