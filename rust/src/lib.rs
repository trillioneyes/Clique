use core::panic;

use godot::engine::{Control, Node, Node2D};
use godot::obj::WithBaseField;
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
#[class(base=Node2D, init)]
struct SampleChildren {
    #[export]
    parent: Option<Gd<Node2D>>,
}

impl SampleChildren {
    fn pick(&self) -> Gd<Node2D> {
        self.parent
            .as_ref()
            .unwrap()
            .get_children()
            .pick_random()
            .unwrap()
            .cast()
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Controller {
    #[export]
    time: f64,
    #[export]
    day: i64,
    #[export]
    time_indicator: Option<Gd<Control>>,
    #[export]
    stockpile: Option<Gd<Node2D>>,
    #[export]
    apple_tree: Option<Gd<SampleChildren>>,
    characters: Vec<Character>,
    apples: i64,
    base: Base<Node>,
}

#[derive(GodotClass)]
#[class(base=Node2D, init)]
struct Traveler {
    #[export]
    speed: f64,
    base: Base<Node2D>,
}

impl Traveler {
    fn with_speed(speed: f64) -> Gd<Self> {
        Gd::from_init_fn(|base| Traveler { speed, base })
    }
}

#[godot_api]
impl INode2D for Traveler {
    fn process(&mut self, delta: f64) {
        let distance = (delta * self.speed) as f32;
        self.base_mut().translate(distance * Vector2::LEFT);
    }
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
            Task::Work => {
                self.pick_apple();
                1
            }
        }
    }

    fn pick_apple(&self) {
        let spawn = self.apple_tree.as_ref().unwrap().bind().pick();
        let scene: Gd<PackedScene> = load("res://apple.tscn");
        let mut apple = scene.instantiate_as::<Traveler>();
        apple.bind_mut().speed = 150.0;
        apple
            .bind_mut()
            .base_mut()
            .set_global_position(spawn.get_global_position());
        self.base().get_parent().unwrap().add_child(apple.upcast())
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
        self.time_indicator.as_mut().map(|ind| {
            ind.call(
                "set_time".into(),
                &[
                    Variant::from(format!("{:?}", new_phase)),
                    Variant::from(format!("{}", self.day)),
                ],
            )
        });
        self.stockpile
            .as_mut()
            .map(|pile| pile.set("apples".into(), Variant::from(self.apples)));
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
            time_indicator: None,
            stockpile: None,
            apple_tree: None,
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
