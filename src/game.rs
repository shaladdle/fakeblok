use log::info;
use piston_window::{context::Context, rectangle, types, G2d, Key};
use rand::Rng;
use serde::{Deserialize, Serialize};
use slab::Slab;

pub type GameInt = f32;
pub type EntityId = usize;
pub struct InvalidKeyError;

const PENDULUM_FORCE: Point = Point::new(54.4, 54.4);
const MOVE_VELOCITY: GameInt = 50.;

fn random_color() -> types::Rectangle<GameInt> {
    let mut rng = rand::thread_rng();
    [0.0, rng.gen(), rng.gen(), rng.gen()]
}

fn random_point(bottom_right: Point) -> Point {
    let mut rng = rand::thread_rng();
    let x: GameInt = rng.gen_range(0., bottom_right.x as f32);
    let y: GameInt = rng.gen_range(0., bottom_right.y as f32);
    Point { x, y }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Pendulum {
        distance: Point,
        max_distance: Point,
    },
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: GameInt,
    pub y: GameInt,
}

impl Point {
    pub const fn new(x: GameInt, y: GameInt) -> Point {
        Point { x, y }
    }

    pub fn is_below(self, other: Point) -> bool {
        self.y > other.y
    }

    pub fn is_right_of(self, other: Point) -> bool {
        self.x > other.x
    }

    pub fn is_origin(self) -> bool {
        self.x == 0. && self.y == 0.
    }

    pub const fn at_x(self, x: GameInt) -> Self {
        Point { x, y: self.y }
    }

    pub const fn at_y(self, y: GameInt) -> Self {
        Point { x: self.x, y }
    }

    /** Element-wise maximum. */
    pub fn max(self, other: Point) -> Self {
        Point {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /** Element-wise minimum. */
    pub fn min(self, other: Point) -> Self {
        Point {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    pub fn copysign(self, sign: Point) -> Self {
        Point {
            x: self.x.copysign(sign.x),
            y: self.y.copysign(sign.y),
        }
    }

    pub fn abs(self) -> Self {
        Point {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            x: self.x.sqrt(),
            y: self.y.sqrt(),
        }
    }

    fn sin(self) -> Self {
        Self {
            x: self.x.sin(),
            y: self.y.sin(),
        }
    }
}

impl std::ops::Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::SubAssign for Point {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl std::ops::Mul<GameInt> for Point {
    type Output = Self;

    fn mul(self, multiplier: GameInt) -> Self {
        Self {
            x: self.x * multiplier,
            y: self.y * multiplier,
        }
    }
}

impl std::ops::Mul for Point {
    type Output = Self;

    fn mul(self, multiplier: Point) -> Self {
        Self {
            x: self.x * multiplier.x,
            y: self.y * multiplier.y,
        }
    }
}

impl std::ops::Div<GameInt> for Point {
    type Output = Self;

    fn div(self, divisor: GameInt) -> Self {
        Self {
            x: self.x / divisor,
            y: self.y / divisor,
        }
    }
}

impl std::ops::Div for Point {
    type Output = Self;

    fn div(self, divisor: Point) -> Self {
        Self {
            x: self.x / divisor.x,
            y: self.y / divisor.y,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Game {
    square_side_length: GameInt,
    pub bottom_right: Point,
    #[serde(with = "serde_slab")]
    pub positions: Slab<Rectangle>,
    #[serde(with = "serde_slab")]
    pub velocities: Slab<Point>,
    #[serde(with = "serde_slab")]
    pub animations: Slab<Option<Animation>>,
    #[serde(with = "serde_slab")]
    pub moveable: Slab<bool>,
    #[serde(with = "serde_slab")]
    pub moved_this_action: Slab<bool>,
    #[serde(with = "serde_slab")]
    pub colors: Slab<types::Rectangle<GameInt>>,
    time: f32,
}

mod serde_slab {
    use serde::{
        de::{MapAccess, Visitor},
        ser::SerializeMap,
        Deserialize, Deserializer, Serialize, Serializer,
    };
    use slab::Slab;
    use std::marker::PhantomData;
    use std::{collections::HashMap, fmt};

    pub fn serialize<T, S>(slab: &Slab<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(slab.capacity()))?;
        for (k, v) in slab.iter() {
            map.serialize_entry(&k, v)?;
        }
        map.end()
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Slab<T>, D::Error>
    where
        T: Deserialize<'de>,
        T: Default,
        D: Deserializer<'de>,
    {
        struct SlabVisitor<T> {
            marker: PhantomData<fn() -> Slab<T>>,
        }
        impl<'de, T> Visitor<'de> for SlabVisitor<T>
        where
            T: Default,
            T: Deserialize<'de>,
        {
            type Value = Slab<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a slab")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut max_value = 0;
                let mut hash_map = HashMap::<usize, _>::new();
                while let Some((key, value)) = access.next_entry()? {
                    hash_map.insert(key, value);
                    max_value = max_value.max(key);
                }

                let mut map = Slab::with_capacity(max_value + 1);
                let mut to_delete = Vec::with_capacity(max_value + 1 - hash_map.len());
                for _ in 0..=max_value {
                    let entry = map.vacant_entry();
                    let key = entry.key();
                    match hash_map.remove(&key) {
                        Some(v) => {
                            entry.insert(v);
                        }
                        None => {
                            // The same key will keep being returned by vacant_entry() unless
                            // we fill it up with something. We just need to delete it later.
                            entry.insert(T::default());
                            to_delete.push(key);
                        }
                    }
                }
                for key in to_delete {
                    map.remove(key);
                }

                assert_eq!(0, hash_map.len());
                Ok(map)
            }
        }
        deserializer.deserialize_map(SlabVisitor {
            marker: PhantomData,
        })
    }
}

pub struct Entity {
    pub position: Rectangle,
    pub velocity: Point,
    pub animation: Option<Animation>,
    pub moveable: bool,
    pub moved_this_action: bool,
    pub color: types::Rectangle<GameInt>,
}

impl Game {
    pub fn new(bottom_right: Point, square_side_length: GameInt) -> Game {
        let mut game = Game {
            square_side_length,
            bottom_right,
            positions: Slab::new(),
            velocities: Slab::new(),
            animations: Slab::new(),
            moveable: Slab::new(),
            moved_this_action: Slab::new(),
            colors: Slab::new(),
            time: 0.,
        };
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let color = random_color();
            let square = Rectangle::new(
                random_point(bottom_right),
                square_side_length / 2.,
                square_side_length / 2.,
            );
            let id = game.insert_entity(Entity {
                position: square,
                velocity: Point::default(),
                animation: None,
                moveable: false,
                moved_this_action: false,
                color,
            });
            game.init_pendulum(id, game.positions[id].top_left + Point::new(-100., 200.));
        }
        for _ in 0..100 {
            let color = random_color();
            let square = Rectangle::new(
                random_point(bottom_right),
                square_side_length / 2.,
                square_side_length / 2.,
            );
            game.insert_entity(Entity {
                position: square,
                velocity: Point::default(),
                animation: None,
                moveable: rng.gen_range(1, 4) == 1,
                moved_this_action: false,
                color,
            });
        }
        game
    }

    pub fn insert_new_player_square(&mut self) -> EntityId {
        let square = Rectangle::new(
            Point::default(),
            self.square_side_length,
            self.square_side_length,
        );
        let color = random_color();
        self.insert_entity(Entity {
            position: square,
            velocity: Point::default(),
            animation: None,
            moveable: true,
            moved_this_action: false,
            color,
        })
    }

    pub fn remove_entity(&mut self, entity: EntityId) {
        self.positions.remove(entity);
        self.velocities.remove(entity);
        self.animations.remove(entity);
        self.moveable.remove(entity);
        self.moved_this_action.remove(entity);
        self.colors.remove(entity);
    }

    pub fn insert_entity(&mut self, entity: Entity) -> EntityId {
        let entity_id = self.positions.insert(entity.position);
        assert_eq!(entity_id, self.velocities.insert(entity.velocity));
        assert_eq!(entity_id, self.animations.insert(entity.animation));
        assert_eq!(entity_id, self.moveable.insert(entity.moveable));
        assert_eq!(
            entity_id,
            self.moved_this_action.insert(entity.moved_this_action)
        );
        assert_eq!(entity_id, self.colors.insert(entity.color));
        entity_id
    }

    fn entity_overlap(&mut self, entity_segments: &[Rectangle], other: EntityId) -> Point {
        entity_segments
            .iter()
            .map(|entity_segment| {
                let mut overlap = Point::default();
                self.positions[other].segments(self.bottom_right, |r| {
                    if let Some(r) = entity_segment.overlap(&r) {
                        overlap = overlap.max(Point::new(r.width, r.height));
                    }
                });
                overlap
            })
            .fold(Point::default(), |first, second| first.max(second))
    }

    pub fn start_move_entity(&mut self, entity: EntityId, delta: Point) -> Point {
        for (_, moved) in &mut self.moved_this_action {
            *moved = false;
        }
        self.move_entity(entity, delta)
    }

    pub fn move_entity(&mut self, entity: EntityId, delta: Point) -> Point {
        self.moved_this_action[entity] = true;
        let game_width = self.width();
        let game_height = self.height();
        let bottom_right = self.bottom_right;
        self.positions[entity].move_(delta, game_width, game_height);
        let mut entity_segments = vec![];
        self.positions[entity].segments(bottom_right, |r| entity_segments.push(r));
        let mut overlap = Point::default();
        for id in 0..self.positions.len() {
            if !self.positions.contains(id) {
                continue;
            }
            if id == entity {
                continue;
            }
            if self.moved_this_action[id] {
                continue;
            }

            let entity_overlap = self.entity_overlap(&entity_segments, id);
            if entity_overlap.x == 0. || entity_overlap.y == 0. {
                continue;
            }
            if self.moveable[id] {
                let to_move = entity_overlap.min(delta.abs()).copysign(delta);
                self.move_entity(id, to_move);
                overlap = overlap.max(self.entity_overlap(&entity_segments, id));
            } else {
                overlap = overlap.max(entity_overlap)
            }
        }
        if overlap.x > 0. && overlap.y > 0. {
            let to_move = overlap.min(delta.abs()).copysign(delta) * -1.;
            self.positions[entity].move_(to_move, game_width, game_height);
        }
        delta - overlap
    }

    pub fn process_key_press(&mut self, id: EntityId, key: &Key) -> Result<(), InvalidKeyError> {
        Ok(match key {
            &Key::W => self.velocities[id].y = -1. * MOVE_VELOCITY,
            &Key::A => self.velocities[id].x = -1. * MOVE_VELOCITY,
            &Key::S => self.velocities[id].y = 1. * MOVE_VELOCITY,
            &Key::D => self.velocities[id].x = 1. * MOVE_VELOCITY,
            _ => return Err(InvalidKeyError),
        })
    }

    pub fn process_key_release(&mut self, id: EntityId, key: &Key) -> Result<(), InvalidKeyError> {
        Ok(match key {
            &Key::W => self.velocities[id].y = 0.,
            &Key::A => self.velocities[id].x = 0.,
            &Key::S => self.velocities[id].y = 0.,
            &Key::D => self.velocities[id].x = 0.,
            _ => return Err(InvalidKeyError),
        })
    }

    fn init_pendulum(&mut self, entity: EntityId, midpoint: Point) {
        let distance = self.positions[entity].top_left - midpoint;
        self.animations[entity] = Some(Animation::Pendulum {
            distance,
            max_distance: distance.abs(),
        });
    }

    pub fn tick(
        &mut self,
        dt: f32,
        time_in_current_bucket: &mut f32,
        ticks_in_current_bucket: &mut i32,
    ) {
        self.time += dt;
        *time_in_current_bucket += dt;
        *ticks_in_current_bucket += 1;
        if *time_in_current_bucket >= 0.25 {
            let ticks_per_sec = *ticks_in_current_bucket as f32 / *time_in_current_bucket;
            if ticks_per_sec < 990. {
                info!("{} ticks/s", ticks_per_sec);
            }
            *time_in_current_bucket = 0.;
            *ticks_in_current_bucket = 0;
        }
        for entity in 0..self.velocities.len() {
            if !self.velocities.contains(entity) {
                continue;
            }
            let mut delta = Point::default();
            if !self.velocities[entity].is_origin() {
                delta += self.start_move_entity(entity, self.velocities[entity].at_y(0.) * dt);
                delta += self.start_move_entity(entity, self.velocities[entity].at_x(0.) * dt);
            }
            match self.animations[entity] {
                Some(Animation::Pendulum {
                    ref mut distance,
                    max_distance,
                }) => {
                    *distance += delta;
                    // I don't know what this is doing but it's kind of interesting.
                    self.velocities[entity] = (max_distance * PENDULUM_FORCE).sqrt()
                        * ((PENDULUM_FORCE / max_distance).sqrt() * self.time).sin();
                }
                None => {}
            }
        }
    }

    pub fn draw(&mut self, pov_id: EntityId, c: Context, g: &mut G2d) {
        let pov = self.positions[pov_id].top_left;
        let pov_width = self.positions[pov_id].width;
        let pov_height = self.positions[pov_id].height;
        let [x, y] = c.get_view_size();
        for (i, entity) in self.positions.iter() {
            let mut entity = entity.clone();
            entity.top_left.x = (entity.top_left.x + self.width() + 0.5 as GameInt * x as GameInt
                - pov.x
                - pov_width / 2.)
                % self.width();
            entity.top_left.y = (entity.top_left.y + self.height() + 0.5 as GameInt * y as GameInt
                - pov.y
                - pov_height / 2.)
                % self.height();
            entity.segments(self.bottom_right, |rect| {
                rectangle(
                    self.colors[i],
                    <_ as Into<types::Rectangle<f64>>>::into(rect),
                    c.transform,
                    g,
                );
            });
        }
    }

    pub fn width(&self) -> GameInt {
        self.bottom_right.x
    }

    pub fn height(&self) -> GameInt {
        self.bottom_right.y
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    pub top_left: Point,
    pub width: GameInt,
    pub height: GameInt,
}

impl Rectangle {
    pub fn new(top_left: Point, width: GameInt, height: GameInt) -> Self {
        Rectangle {
            top_left,
            width,
            height,
        }
    }

    pub fn move_(&mut self, diff: Point, width: GameInt, height: GameInt) {
        self.top_left.x = (width + self.top_left.x + (diff.x % width)) % width;
        self.top_left.y = (height + self.top_left.y + (diff.y % height)) % height;
    }

    pub fn overlap(&self, other: &Rectangle) -> Option<Rectangle> {
        let self_bottom_right = self.bottom_right();
        let other_bottom_right = other.bottom_right();
        if self.top_left.x < other_bottom_right.x
            && self.top_left.y < other_bottom_right.y
            && other.top_left.x < self_bottom_right.x
            && other.top_left.y < self_bottom_right.y
        {
            let x = self.top_left.x.max(other.top_left.x);
            let y = self.top_left.y.max(other.top_left.y);
            Some(Rectangle {
                top_left: Point { x, y },
                width: self_bottom_right.x.min(other_bottom_right.x) - x,
                height: self_bottom_right.y.min(other_bottom_right.y) - y,
            })
        } else {
            None
        }
    }

    pub fn segments(&self, bottom_right: Point, mut f: impl FnMut(Rectangle)) {
        self.segments_helper(bottom_right, &mut f);
    }

    fn segments_helper(&self, bottom_right: Point, f: &mut impl FnMut(Rectangle)) {
        let my_bottom_right = self.bottom_right();
        if my_bottom_right.is_below(bottom_right) {
            let bottom_overflow = my_bottom_right.y - bottom_right.y;
            Rectangle {
                top_left: Point {
                    x: self.top_left.x,
                    y: 0.,
                },
                width: self.width,
                height: bottom_overflow,
            }
            .segments_helper(bottom_right, f);
        }
        if my_bottom_right.is_right_of(bottom_right) {
            let right_overflow = my_bottom_right.x - bottom_right.x;
            Rectangle {
                top_left: Point {
                    x: 0.,
                    y: self.top_left.y,
                },
                width: right_overflow,
                height: self.height,
            }
            .segments_helper(bottom_right, f);
        }
        f(Rectangle {
            top_left: self.top_left,
            width: self.width.min(bottom_right.x - self.top_left.x),
            height: self.height.min(bottom_right.y - self.top_left.y),
        });
    }

    pub fn bottom_right(&self) -> Point {
        self.top_left
            + Point {
                x: self.width,
                y: self.height,
            }
    }
}

impl Into<types::Rectangle<f64>> for Rectangle {
    fn into(self) -> types::Rectangle<f64> {
        [
            self.top_left.x as f64,
            self.top_left.y as f64,
            self.width as f64,
            self.height as f64,
        ]
    }
}

#[test]
fn my_rectangle_segments_no_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5., y: 5. },
        width: 5.,
        height: 5.,
    };
    let mut expected_recs = vec![Rectangle::new(Point::new(5., 5.), 5., 5.)];
    rect.segments(Point { x: 10., y: 10. }, |r| {
        for (i, rec) in expected_recs.iter() {
            if rec == &r {
                expected_recs.remove(i);
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}

#[test]
fn my_rectangle_segments_overflow() {
    let rect = Rectangle {
        top_left: Point { x: 5., y: 5. },
        width: 5.,
        height: 5.,
    };
    let mut expected_recs = vec![
        Rectangle::new(Point::new(5., 5.), 2., 5.),
        Rectangle::new(Point::new(0., 5.), 3., 5.),
    ];
    rect.segments(Point { x: 7., y: 10. }, |r| {
        for (i, rec) in expected_recs.iter() {
            if rec == &r {
                expected_recs.remove(i);
                return;
            }
        }
        panic!("Expected one of {:?}; got {:?}", expected_recs, r);
    });
}

#[test]
fn rectangle_move() {
    let mut rect = Rectangle {
        top_left: Point { x: 5., y: 5. },
        width: 5.,
        height: 5.,
    };
    rect.move_(Point::new(5., 5.), 10., 10.);
    assert_eq!(rect, Rectangle::new(Point::default(), 5., 5.));
    rect.move_(Point::new(-5., -5.), 10., 10.);
    assert_eq!(rect, Rectangle::new(Point::new(5., 5.), 5., 5.));
}
