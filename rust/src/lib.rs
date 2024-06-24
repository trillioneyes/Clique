use std::cell::OnceCell;
use std::collections::VecDeque;
use std::rc::Rc;

use godot::engine::{Control, Node, Node2D};
use godot::obj::WithBaseField;
use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

struct Character(Gd<Node2D>);

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

#[derive(PartialEq, Debug, Clone, Copy)]
enum SubPhase {
    Commence,
    Progress,
    Complete,
    Tempo,
}

impl SubPhase {
    fn next(self) -> Self {
        match self {
            SubPhase::Commence => SubPhase::Progress,
            SubPhase::Progress => SubPhase::Complete,
            SubPhase::Complete => SubPhase::Tempo,
            SubPhase::Tempo => SubPhase::Commence,
        }
    }
}

struct GameTime {
    day: i64,
    phase: Phase,
    sub: SubPhase,
}

impl GameTime {
    fn start() -> Self {
        GameTime {
            day: 1,
            phase: Phase::Predawn,
            sub: SubPhase::Tempo,
        }
    }

    fn next(&mut self) {
        self.sub = self.sub.next();
        if self.sub == SubPhase::Commence {
            self.phase = self.phase.next();
            if self.phase == Phase::Predawn {
                self.day += 1;
            }
        }
    }
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

#[derive(Clone)]
enum Outcome {
    StatusQuo,
    Apples { delta: i64 },
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::StatusQuo
    }
}

#[derive(Default, Clone)]
struct OutcomeChannel {
    cell: Rc<OnceCell<Outcome>>,
    outcome: Outcome,
}

impl OutcomeChannel {
    fn new(outcome: Outcome) -> Self {
        OutcomeChannel {
            cell: Rc::new(OnceCell::new()),
            outcome,
        }
    }

    fn check(self) -> (Option<Outcome>, Option<Self>) {
        if let Some(outcome) = self.cell.get() {
            (Some(outcome.clone()), None)
        } else {
            (None, Some(self))
        }
    }

    fn fire(&self) {
        let _ = self.cell.set(self.outcome.clone());
    }

    fn from(outcome: Outcome) -> Self {
        OutcomeChannel {
            cell: Rc::new(OnceCell::from(outcome.clone())),
            outcome,
        }
    }
}

struct OutcomeMux {
    channels: Vec<OutcomeChannel>,
}

impl OutcomeMux {
    fn tick(self) -> (Vec<Outcome>, Option<Self>) {
        let mut done: Vec<Outcome> = vec![];
        let mut remaining: Vec<OutcomeChannel> = vec![];
        self.channels.into_iter().for_each(|channel| {
            let (outcome, rest) = channel.check();
            outcome.map(|outcome| done.push(outcome));
            rest.map(|rest| remaining.push(rest));
        });
        (
            done,
            if remaining.is_empty() {
                None
            } else {
                Some(OutcomeMux {
                    channels: remaining,
                })
            },
        )
    }

    fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    fn from(channels: impl IntoIterator<Item = OutcomeChannel>) -> Self {
        OutcomeMux {
            channels: channels.into_iter().collect(),
        }
    }
}

enum Item {
    Wait { seconds: f64 },
    Play(OutcomeMux),
}

impl Item {
    fn tick(self, delta: f64) -> (Vec<Outcome>, Option<Self>) {
        match self {
            Item::Wait { seconds } => {
                if seconds >= delta {
                    (
                        vec![],
                        Some(Item::Wait {
                            seconds: seconds - delta,
                        }),
                    )
                } else {
                    (vec![], None)
                }
            }
            Item::Play(outcomes) => match outcomes.tick() {
                (done, left) => (done, left.map(Item::Play)),
            },
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node, no_init)]
struct Controller {
    time: GameTime,
    queue: VecDeque<Item>,
    time_indicator: Gd<Control>,
    stockpile: Gd<Node2D>,
    apple_tree: Gd<SampleChildren>,
    characters: Vec<Character>,
    apples: i64,
    base: Base<Node>,
}

#[derive(GodotClass)]
#[class(base=Node, init)]
struct Cyst {
    #[export]
    time_indicator: Option<Gd<Control>>,
    #[export]
    stockpile: Option<Gd<Node2D>>,
    #[export]
    apple_tree: Option<Gd<SampleChildren>>,
    base: Base<Node>,
}

impl Cyst {
    fn parts(&mut self) -> Option<(Gd<Control>, Gd<Node2D>, Gd<SampleChildren>)> {
        self.time_indicator.take().and_then(|time| {
            self.stockpile.take().and_then(|stock| {
                self.apple_tree
                    .take()
                    .and_then(|apples| Option::Some((time, stock, apples)))
            })
        })
    }
}

#[derive(GodotClass)]
#[class(base=Node2D, init)]
struct Traveler {
    velocity: Vector2,
    target: Vector2,
    signal: OutcomeChannel,
    base: Base<Node2D>,
}

impl Traveler {
    fn new(speed: f32, result: Outcome, from: &Node2D, to: &Node2D) -> Gd<Self> {
        let start = from.get_global_position();
        let end = to.get_global_position();
        let velocity = (end - start).normalized() * speed;
        let mut traveler = Gd::from_init_fn(|base| Traveler {
            velocity,
            signal: OutcomeChannel::new(result),
            target: end,
            base,
        });
        traveler.set_global_position(start);
        traveler
    }
}

#[godot_api]
impl INode2D for Traveler {
    fn process(&mut self, delta: f64) {
        let displacement = delta as f32 * self.velocity;
        let new_pos = self
            .base()
            .get_global_position()
            .move_toward(self.target, displacement.length());
        self.base_mut().set_global_position(new_pos);
        if new_pos == self.target {
            self.signal.fire();
            self.base_mut().queue_free()
        }
    }
}

impl Controller {
    fn new(cyst: &mut Cyst) -> Option<Gd<Self>> {
        cyst.parts().map(|(time, stock, tree)| {
            Gd::from_init_fn(|base| Self {
                queue: VecDeque::with_capacity(4),
                time: GameTime::start(),
                characters: vec![],
                apples: 0,
                base,
                time_indicator: time,
                stockpile: stock,
                apple_tree: tree,
            })
        })
    }

    fn fulfill(&self, character: &Character, task: Task) -> OutcomeChannel {
        match task {
            Task::Eat => OutcomeChannel::from(Outcome::StatusQuo),
            Task::Sleep => OutcomeChannel::from(Outcome::StatusQuo),
            Task::Work => {
                let traveler = self.pick_apple(character);
                let outcome = traveler.bind().signal.clone();
                outcome
            }
        }
    }

    fn finish(&self, character: &Character, task: Task) -> OutcomeChannel {
        match task {
            Task::Eat => OutcomeChannel::from(Outcome::Apples { delta: -1 }),
            Task::Sleep => OutcomeChannel::from(Outcome::StatusQuo),
            Task::Work => {
                let traveler = self.store_apple(character);
                let outcome = traveler.bind().signal.clone();
                outcome
            }
        }
    }

    fn apply(&mut self, o: &Outcome) {
        match o {
            Outcome::StatusQuo => (),
            Outcome::Apples { delta } => self.apples += delta,
        }
        self.stockpile
            .set("apples".into(), Variant::from(self.apples));
    }

    fn pick_apple(&self, character: &Character) -> Gd<Traveler> {
        let spawn = self.apple_tree.bind().pick();
        let scene: Gd<PackedScene> = load("res://apple.tscn");
        let apple = scene.instantiate_as::<Node2D>();
        let mut traveler = Traveler::new(400.0, Outcome::StatusQuo, &spawn, &character.0);
        traveler.add_child(apple.upcast());
        self.base()
            .get_parent()
            .unwrap()
            .add_child(traveler.clone().upcast());
        traveler
    }

    fn store_apple(&self, character: &Character) -> Gd<Traveler> {
        let scene: Gd<PackedScene> = load("res://apple.tscn");
        let apple = scene.instantiate_as::<Node2D>();
        let mut traveler = Traveler::new(
            1000.0,
            Outcome::Apples { delta: 1 },
            &character.0,
            &self.stockpile,
        );
        traveler.add_child(apple.upcast());
        self.base()
            .get_parent()
            .unwrap()
            .add_child(traveler.clone().upcast());
        traveler
    }

    fn character_actions(&self) -> Item {
        let mut actions = vec![];
        for c in self.characters.iter() {
            let task = c.decide(self.time.phase);
            actions.push(self.fulfill(c, task));
        }
        Item::Play(OutcomeMux::from(actions))
    }

    fn character_cleanup(&self) -> Item {
        let mut cleanups = vec![];
        for c in self.characters.iter() {
            let task = c.decide(self.time.phase);
            cleanups.push(self.finish(c, task));
        }
        Item::Play(OutcomeMux::from(cleanups))
    }

    fn schedule_item(&self) -> Item {
        match self.time.sub {
            SubPhase::Commence => self.character_actions(),
            SubPhase::Complete => self.character_cleanup(),
            _ => Item::Wait { seconds: 0.5 },
        }
    }
}

#[godot_api]
impl INode for Controller {
    fn process(&mut self, delta: f64) {
        let current = self.queue.pop_front();
        match current {
            None => self.queue.push_back(self.schedule_item()),
            Some(current) => {
                let (outcomes, next) = current.tick(delta);
                for outcome in &outcomes {
                    self.apply(outcome)
                }
                match next {
                    Some(next) => self.queue.push_front(next),
                    None => self.time.next(),
                }
                self.time_indicator.call(
                    "set_time".into(),
                    &[
                        Variant::from(format!("{:?}", self.time.phase)),
                        Variant::from(format!("{}", self.time.day)),
                    ],
                );
            }
        }
    }

    fn enter_tree(&mut self) {
        self.base()
            .get_tree()
            .unwrap()
            .get_nodes_in_group("characters".into())
            .iter_shared()
            .for_each(|node| self.characters.push(Character(node.cast())));
    }
}

#[godot_api]
impl INode for Cyst {
    fn enter_tree(&mut self) {
        Controller::new(self).map(|controller| self.base_mut().add_child(controller.upcast()));
    }
}
