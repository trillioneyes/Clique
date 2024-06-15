use core::panic;

use godot::classes::Sprite2D;
use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

#[derive(PartialEq, Debug)]
enum Phase {
    Morning,
    Midday,
    Evening,
    Night,
}

impl Phase {
    fn next(&self) -> Self {
        match self {
            Phase::Morning => Phase::Midday,
            Phase::Midday => Phase::Evening,
            Phase::Evening => Phase::Night,
            Phase::Night => Phase::Morning,
        }
    }

    fn from_index(i: i64) -> Self {
        match i % 4 {
            0 => Phase::Morning,
            1 => Phase::Midday,
            2 => Phase::Evening,
            3 => Phase::Night,
            _ => panic!("Impossible"),
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Controller {
    #[export]
    time: f64,
    base: Base<Node>,
}

impl Controller {
    fn phase(&self) -> Phase {
        Phase::from_index((self.time / 10.0) as i64)
    }

    fn sun_transit(&self, old_phase: Phase, new_phase: Phase) {
        godot_print!("Phase transition from {:?} to {:?}!", old_phase, new_phase)
    }
}

#[godot_api]
impl INode for Controller {
    fn init(base: Base<Node>) -> Self {
        Self { time: 0.0, base }
    }

    fn process(&mut self, delta: f64) {
        let old = self.phase();
        self.time += delta;
        if old != self.phase() {
            self.sun_transit(old, self.phase())
        }
    }
}
