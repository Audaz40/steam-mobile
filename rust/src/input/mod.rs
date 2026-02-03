//! Input handling module
//!
//! Provides touch overlays, controller mappings, and unified input actions.

use std::collections::HashMap;

/// Input action emitted by the input system.
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    KeyPress { keycode: u32 },
    KeyRelease { keycode: u32 },
    MouseMove { dx: f32, dy: f32 },
    MouseButton { button: u8, pressed: bool },
    Axis { axis: u8, value: f32 },
}

/// Basic touch point data.
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

/// A rectangular touch region used for virtual controls.
#[derive(Debug, Clone, Copy)]
pub struct TouchRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl TouchRegion {
    pub fn contains(&self, point: TouchPoint) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// Virtual button configuration.
#[derive(Debug, Clone)]
pub struct VirtualButton {
    pub id: u32,
    pub label: String,
    pub region: TouchRegion,
    pub keycode: u32,
}

/// Virtual analog stick configuration.
#[derive(Debug, Clone)]
pub struct VirtualStick {
    pub id: u32,
    pub region: TouchRegion,
    pub dead_zone: f32,
    pub axis_x: u8,
    pub axis_y: u8,
}

/// Touch overlay configuration.
#[derive(Debug, Clone)]
pub struct InputOverlay {
    pub buttons: Vec<VirtualButton>,
    pub sticks: Vec<VirtualStick>,
}

impl InputOverlay {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            sticks: Vec::new(),
        }
    }

    pub fn add_button(&mut self, button: VirtualButton) {
        self.buttons.push(button);
    }

    pub fn add_stick(&mut self, stick: VirtualStick) {
        self.sticks.push(stick);
    }

    pub fn map_touch(&self, touch: TouchPoint) -> Vec<InputAction> {
        let mut actions = Vec::new();
        for button in &self.buttons {
            if button.region.contains(touch) {
                actions.push(InputAction::KeyPress {
                    keycode: button.keycode,
                });
            }
        }

        for stick in &self.sticks {
            if stick.region.contains(touch) {
                let center_x = stick.region.x + stick.region.width / 2.0;
                let center_y = stick.region.y + stick.region.height / 2.0;
                let dx = (touch.x - center_x) / (stick.region.width / 2.0);
                let dy = (touch.y - center_y) / (stick.region.height / 2.0);
                let magnitude = (dx * dx + dy * dy).sqrt();

                if magnitude > stick.dead_zone {
                    actions.push(InputAction::Axis {
                        axis: stick.axis_x,
                        value: dx.clamp(-1.0, 1.0),
                    });
                    actions.push(InputAction::Axis {
                        axis: stick.axis_y,
                        value: dy.clamp(-1.0, 1.0),
                    });
                }
            }
        }

        actions
    }
}

/// Controller button mapping.
#[derive(Debug, Clone)]
pub struct ControllerMapping {
    pub button_map: HashMap<u32, InputAction>,
    pub axis_map: HashMap<u8, u8>,
}

impl ControllerMapping {
    pub fn new() -> Self {
        Self {
            button_map: HashMap::new(),
            axis_map: HashMap::new(),
        }
    }

    pub fn map_button(&mut self, button_id: u32, action: InputAction) {
        self.button_map.insert(button_id, action);
    }

    pub fn map_axis(&mut self, input_axis: u8, target_axis: u8) {
        self.axis_map.insert(input_axis, target_axis);
    }
}

/// Input manager that aggregates touch and controller input.
pub struct InputManager {
    overlay: InputOverlay,
    controller_mappings: HashMap<u32, ControllerMapping>,
    queued_actions: Vec<InputAction>,
}

impl InputManager {
    pub fn new(overlay: InputOverlay) -> Self {
        Self {
            overlay,
            controller_mappings: HashMap::new(),
            queued_actions: Vec::new(),
        }
    }

    pub fn set_overlay(&mut self, overlay: InputOverlay) {
        self.overlay = overlay;
    }

    pub fn register_controller(&mut self, controller_id: u32, mapping: ControllerMapping) {
        self.controller_mappings.insert(controller_id, mapping);
    }

    pub fn handle_touch(&mut self, touch: TouchPoint) {
        let actions = self.overlay.map_touch(touch);
        self.queued_actions.extend(actions);
    }

    pub fn handle_controller_button(&mut self, controller_id: u32, button_id: u32, pressed: bool) {
        if let Some(mapping) = self.controller_mappings.get(&controller_id) {
            if let Some(action) = mapping.button_map.get(&button_id) {
                let mut mapped_action = action.clone();
                if let InputAction::KeyPress { keycode } = action {
                    mapped_action = if pressed {
                        InputAction::KeyPress { keycode: *keycode }
                    } else {
                        InputAction::KeyRelease { keycode: *keycode }
                    };
                }
                self.queued_actions.push(mapped_action);
            }
        }
    }

    pub fn handle_controller_axis(&mut self, controller_id: u32, axis_id: u8, value: f32) {
        if let Some(mapping) = self.controller_mappings.get(&controller_id) {
            if let Some(target_axis) = mapping.axis_map.get(&axis_id) {
                self.queued_actions.push(InputAction::Axis {
                    axis: *target_axis,
                    value,
                });
            }
        }
    }

    pub fn drain_actions(&mut self) -> Vec<InputAction> {
        let actions = self.queued_actions.clone();
        self.queued_actions.clear();
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_button_mapping() {
        let mut overlay = InputOverlay::new();
        overlay.add_button(VirtualButton {
            id: 1,
            label: "A".to_string(),
            region: TouchRegion {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            keycode: 65,
        });

        let mut manager = InputManager::new(overlay);
        manager.handle_touch(TouchPoint {
            x: 50.0,
            y: 50.0,
            pressure: 0.8,
        });

        let actions = manager.drain_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], InputAction::KeyPress { keycode: 65 });
    }
}
