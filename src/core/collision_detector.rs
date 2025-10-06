use crate::core::vehicle_data::Vehicle;
use crate::direction::TurnDirection;
use crate::geometry::position::Position;

pub struct CollisionDetector;

impl CollisionDetector {
    pub fn is_relevant_for_collision(
        self_vehicle: &Vehicle,
        other_vehicle: &Vehicle,
        current_position: &Position,
        time: &u64,
    ) -> bool {
        let same_lane = self_vehicle.initial_position == other_vehicle.initial_position
            && self_vehicle.target_direction == other_vehicle.target_direction;

        if (self_vehicle.turn_direction == TurnDirection::Right
            || other_vehicle.turn_direction == TurnDirection::Right)
            && !same_lane
        {
            return false;
        }

        if self_vehicle.start_direction == other_vehicle.start_direction
            && self_vehicle.target_direction != other_vehicle.target_direction
        {
            return false;
        }

        if self_vehicle.turn_direction == TurnDirection::Straight
            && other_vehicle.turn_direction == TurnDirection::Straight
            && self_vehicle.initial_position == other_vehicle.start_direction
        {
            return false;
        }

        if !same_lane && !current_position.is_in_intersection() {
            return false;
        }

        if !other_vehicle.path.iter().any(|tp| tp.time == *time) {
            return false;
        }

        true
    }
}
