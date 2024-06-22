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

type OutcomeChannel = Rc<OnceCell<Outcome>>;

enum Item {
    Wait { seconds: f64 },
    Play(Vec<OutcomeChannel>),
}

impl Item {
    fn finished(&self) -> bool {
        match self {
            Item::Wait { seconds } => *seconds <= 0.0,
            Item::Play(cells) => cells.iter().all(|cell| cell.get().is_some()),
        }
    }

    fn tick(self, delta: f64) -> (Vec<Outcome>, Self) {
        match self {
            Item::Wait { seconds } => (
                vec![],
                Item::Wait {
                    seconds: seconds - delta,
                },
            ),
            Item::Play(outcomes) => {
                let (done, not): (Vec<OutcomeChannel>, Vec<OutcomeChannel>) = outcomes
                    .into_iter()
                    .partition(|outcome_cell| outcome_cell.get().is_some());
                let done = done
                    .into_iter()
                    .map(|outcome_cell| outcome_cell.get().unwrap().clone())
                    .collect();
                (done, Item::Play(not))
            }
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Controller {
    time: GameTime,
    queue: VecDeque<Item>,
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
    velocity: Vector2,
    target: Vector2,
    result: Outcome,
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
            result,
            signal: Rc::new(OnceCell::new()),
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
            let _ = self.signal.set(self.result.clone());
            self.base_mut().queue_free()
        }
    }
}

impl Controller {
    fn fulfill(&self, character: &Character, task: Task) -> OutcomeChannel {
        match task {
            Task::Eat => Rc::new(OnceCell::from(Outcome::StatusQuo)),
            Task::Sleep => Rc::new(OnceCell::from(Outcome::StatusQuo)),
            Task::Work => {
                let traveler = self.pick_apple(character);
                let outcome = traveler.bind().signal.clone();
                outcome
            }
        }
    }

    fn finish(&self, character: &Character, task: Task) -> OutcomeChannel {
        match task {
            Task::Eat => Rc::new(OnceCell::from(Outcome::Apples { delta: -1 })),
            Task::Sleep => Rc::new(OnceCell::from(Outcome::StatusQuo)),
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
            .as_mut()
            .unwrap()
            .set("apples".into(), Variant::from(self.apples));
    }

    fn pick_apple(&self, character: &Character) -> Gd<Traveler> {
        let spawn = self.apple_tree.as_ref().unwrap().bind().pick();
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
            self.stockpile.as_ref().unwrap(),
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
        Item::Play(actions)
    }

    fn character_cleanup(&self) -> Item {
        let mut cleanups = vec![];
        for c in self.characters.iter() {
            let task = c.decide(self.time.phase);
            cleanups.push(self.finish(c, task));
        }
        Item::Play(cleanups)
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
    fn init(base: Base<Node>) -> Self {
        Self {
            queue: VecDeque::with_capacity(4),
            time: GameTime::start(),
            characters: vec![],
            apples: 0,
            base,
            time_indicator: None,
            stockpile: None,
            apple_tree: None,
        }
    }

    fn process(&mut self, delta: f64) {
        let current = self.queue.pop_front();
        match current {
            None => self.queue.push_back(self.schedule_item()),
            Some(current) => {
                let (outcomes, current) = current.tick(delta);
                for outcome in &outcomes {
                    self.apply(outcome)
                }
                if !current.finished() {
                    self.queue.push_front(current);
                } else {
                    self.time.next();
                    match current {
                        Item::Play(outcomes) => outcomes
                            .iter()
                            .filter_map(|rc_cell| rc_cell.get())
                            .for_each(|outcome| self.apply(outcome)),
                        _ => (),
                    }
                    self.time_indicator.as_mut().map(|ind| {
                        ind.call(
                            "set_time".into(),
                            &[
                                Variant::from(format!("{:?}", self.time.phase)),
                                Variant::from(format!("{}", self.time.day)),
                            ],
                        )
                    });
                }
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
