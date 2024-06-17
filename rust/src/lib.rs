use core::panic;

use godot::engine::Node;
use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

struct Character();

enum Task {
    Eat,
    Sleep,
    Work,
}

impl Character {
    fn decide(&self, phase: Phase) -> Task {
        match phase {
            Phase::Predawn => Task::Sleep,
            Phase::Morning => Task::Work,
            Phase::Midday => Task::Eat,
            Phase::Evening => Task::Work,
            Phase::Night => Task::Sleep,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Phase {
    Predawn,
    Morning,
    Midday,
    Evening,
    Night,
}

impl Phase {
    fn next(&self) -> Self {
        match self {
            Phase::Predawn => Phase::Morning,
            Phase::Morning => Phase::Midday,
            Phase::Midday => Phase::Evening,
            Phase::Evening => Phase::Night,
            Phase::Night => Phase::Predawn,
        }
    }

    fn from_index(i: i64) -> Self {
        match i % 5 {
            0 => Phase::Predawn,
            1 => Phase::Morning,
            2 => Phase::Midday,
            3 => Phase::Evening,
            4 => Phase::Night,
            _ => panic!("Impossible"),
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Controller {
    #[export]
    time: f64,
    #[export]
    day: i64,
    characters: Vec<Character>,
    apples: i64,
    base: Base<Node>,
}

impl Controller {
    fn phase(&self) -> Phase {
        Phase::from_index((self.time / 5.0) as i64)
    }

    fn fulfill(&self, _character: &Character, task: Task) -> i64 {
        match task {
            Task::Eat => -1,
            Task::Sleep => {
                godot_print!("Zzzzz");
                0
            }
            Task::Work => 1,
        }
    }

    fn sun_transit(&mut self, old_phase: Phase, new_phase: Phase) {
        if new_phase == Phase::Predawn {
            self.day += 1;
        } else if new_phase == Phase::Morning {
            godot_print!("The sun rises on day {}!", self.day)
        }
        let mut delta_apples = 0;
        for c in self.characters.iter() {
            let task = c.decide(old_phase);
            delta_apples += self.fulfill(c, task);
        }
        self.apples += delta_apples;
        godot_print!(
            "Phase transition from {:?} to {:?}: apple stockpile at {}.",
            old_phase,
            new_phase,
            self.apples
        )
    }
}

#[godot_api]
impl INode for Controller {
    fn init(base: Base<Node>) -> Self {
        Self {
            time: 0.0,
            day: 0,
            characters: vec![Character(), Character(), Character()],
            apples: 0,
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let old = self.phase();
        self.time += delta;
        if old != self.phase() {
            self.sun_transit(old, self.phase())
        }
    }
}
