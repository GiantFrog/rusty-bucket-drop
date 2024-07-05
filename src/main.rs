use std::fmt;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::window::ExitCondition;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::AudioSource as AudioSource;
use leafwing_input_manager::prelude::*;
use rand::prelude::*;


//     COMPONENTS & STUFF     \\
#[derive(Component)]
struct Bucket;

#[derive(Resource)]
struct GameStats {
    score: i128
}
impl Default for GameStats {
    fn default() -> Self {
        Self {
            score: 0
        }
    }
}
impl GameStats {
    fn new() -> Self { Default::default() }
}

#[derive(Component)]
struct Speed {
    horizontal: f32,
    vertical: f32
}
impl Default for Speed {
    fn default() -> Self {
        Self {
            horizontal: 0.0,
            vertical: 0.0
        }
    }
}

#[derive(Component)]
enum DropletType {
    Raindrop,
    Stone,
    Sponge
}
impl fmt::Display for DropletType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DropletType::Raindrop => write!(f, "raindrop"),
            DropletType::Stone => write!(f, "stone"),
            DropletType::Sponge => write!(f, "sponge")
        }
    }
}

#[derive(Resource)]
struct Sprites {
    bucket: Handle<Image>,
    droplet: Handle<Image>,
    stone: Handle<Image>,
    sponge: Handle<Image>
}

#[derive(Resource)]
struct Sfx {
    drop: [Handle<AudioSource>; 3],
    splash: [Handle<AudioSource>; 3],
    tink: [Handle<AudioSource>; 3]
}

#[derive(Bundle)]
struct BucketBundle {
    bucket: Bucket,
    input: InputManagerBundle<BucketAction>,
    speed: Speed,
    sprite: SpriteBundle
}
impl BucketBundle {
    fn default_input_map() -> InputMap<BucketAction> {
        use BucketAction::*;
        // Describes how to convert from player inputs into actions
        let mut input_map = InputMap::default();
        input_map.insert(KeyCode::Left, Left);
        input_map.insert(QwertyScanCode::A, Left);
        input_map.insert(GamepadButtonType::DPadLeft, Left);
        input_map.insert(KeyCode::Right, Right);
        input_map.insert(QwertyScanCode::D, Right);
        input_map.insert(GamepadButtonType::DPadRight, Right);

        return input_map
    }
}

#[derive(Bundle)]
struct FallingThing {
    droplet_type: DropletType,
    speed: Speed,
    sprite: SpriteBundle,
    tracker: CollisionTracker
}
impl Default for FallingThing {
    fn default() -> Self {
        Self {
            droplet_type: DropletType::Raindrop,
            sprite: Default::default(),
            speed: Speed {
                horizontal: 0.0,
                vertical: -200.0
            },
            tracker: Default::default()
        }
    }
}

#[derive(Resource)]
struct DropTimer(Timer);

#[derive(Component)]
struct CollisionTracker {
    time: Option<Timer>,
    force: f32
}
impl Default for CollisionTracker {
    fn default() -> Self {
        Self {
            time: None,
            force: 0.0
        }
    }
}
impl CollisionTracker {
    fn new() -> CollisionTracker {
        let mut timer = Timer::from_seconds(0.2, TimerMode::Once);
        timer.pause();
        return CollisionTracker {
            time: Some(timer),
            ..default()
        }
    }
    fn get_fields(&mut self) -> (&mut Option<Timer>, &mut f32) {
        return (&mut self.time, &mut self.force)
    }
}

#[derive(Component)]
struct WaterLevel {
    current: i16,
    max: i16
}
impl WaterLevel {
    fn add_water(&mut self) {
        self.current += 10;
    }
    fn remove_water(&mut self) {
        self.current = (self.current-10).clamp(0, self.max);
    }
    fn overflowing(&self) -> bool {
        self.current > self.max
    }
}

#[derive(Bundle)]
struct WaterBundle {
    sprite: SpriteBundle,
    water_level: WaterLevel
}
impl Default for WaterBundle {
    fn default() -> Self {
        Self {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.0, 0.0, 0.5, 0.99),
                    custom_size: Some(Vec2::new(800.0, 0.0)),
                    ..default()
                },
                //-240 sends it off the screen so it never gets rendered no matter how large it is, I guess.
                transform: Transform::from_translation(Vec3::new(0.0, -239.9, 3.0)),
                ..default()
            },
            water_level: WaterLevel {
                current: 0,
                max: 70
            }
        }
    }
}


//           SETUP            \\
fn main() {
    let mut wgpu_settings = WgpuSettings::default();
    wgpu_settings.backends = Some(Backends::VULKAN);
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.2)))
        .add_plugins((DefaultPlugins
            .set(RenderPlugin {
                render_creation: wgpu_settings.into()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (800.0, 480.0).into(),
                    title:"Drop".to_string(),
                    ..default()
                }),
                exit_condition: ExitCondition::OnPrimaryClosed,
                close_when_requested: true
            }),
            AudioPlugin,
            DropPlugin
        ))
        .run();
}

pub struct DropPlugin;
impl Plugin for DropPlugin {
    fn build(&self, drop: &mut App) {
        drop.insert_resource(GameStats::new())
            .insert_resource(DropTimer(Timer::from_seconds(0.9, TimerMode::Repeating)))
            .add_plugins(InputManagerPlugin::<BucketAction>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (drop_object, move_objects, process_falling_things, move_bucket));
    }
}

fn setup(mut commands: Commands, server: Res<AssetServer>, audio: Res<Audio>) {
    //get assets loading from disk right away
    let bucket: Handle<Image> = server.load("bucket.png");
    let droplet: Handle<Image> = server.load("droplet.png");
    let stone: Handle<Image> = server.load("stone.png");
    let sponge: Handle<Image> = server.load("sponge.png");
    let sprites = Sprites { bucket, droplet, stone, sponge };
    
    let drop1: Handle<AudioSource> = server.load("sfx/drop1.wav");
    let drop2: Handle<AudioSource> = server.load("sfx/drop2.wav");
    let drop3: Handle<AudioSource> = server.load("sfx/drop3.mp3");
    let splash1: Handle<AudioSource> = server.load("sfx/splash1.mp3");
    let splash2: Handle<AudioSource> = server.load("sfx/splash2.mp3");
    let splash3: Handle<AudioSource> = server.load("sfx/splash3.mp3");
    let tink1: Handle<AudioSource> = server.load("sfx/tink1.mp3");
    let tink2: Handle<AudioSource> = server.load("sfx/tink2.mp3");
    let tink3: Handle<AudioSource> = server.load("sfx/tink3.mp3");
    let sfx = Sfx {
        drop: [drop1, drop2, drop3],
        splash: [splash1, splash2, splash3],
        tink: [tink1, tink2, tink3]
    };

    //camera
    commands.spawn(Camera2dBundle::default());
    //water
    commands.spawn(WaterBundle::default());
    //bucket
    commands.spawn(BucketBundle {
        bucket: Bucket,
        input: InputManagerBundle {
            input_map: BucketBundle::default_input_map(),
            ..default()
        },
        speed: Default::default(),
        sprite: SpriteBundle {
            texture: sprites.bucket.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, -188.0, 0.0)),
            ..default()
        }
    });
    //chocolate rain
    audio.play(server.load("ChocolateRain.mp3")).with_volume(0.2).looped();

    // now that we're done with them, we move the handles to a resource:
    //  - to prevent the asset from being unloaded
    //  - if we want to use it to access the asset later
    commands.insert_resource(sprites);
    commands.insert_resource(sfx);
}


//         LOOP STUFF         \\
fn drop_object(mut commands: Commands, time: Res<Time>, mut timer: ResMut<DropTimer>, sprites: Res<Sprites>) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = thread_rng();
        let randy: f32 = rng.gen::<f32>();
        //random position from -400 to 400, with 32 pixels of buffer on each side.
        let pos: f32 = rng.gen::<f32>()*736.0 - 368.0;
        if randy >= 0.95 {          //5% chance for a sponge
            commands.spawn(FallingThing {
                droplet_type: DropletType::Sponge,
                sprite: SpriteBundle {
                    texture: sprites.sponge.clone(),
                    transform: Transform::from_translation(Vec3::new(pos, 300.0, 1.0)),
                    ..default()
                },
                ..Default::default()
            });
        } else if randy >= 0.75 {   //20% chance for a stone
            commands.spawn(FallingThing {
                droplet_type: DropletType::Stone,
                sprite: SpriteBundle {
                    texture: sprites.stone.clone(),
                    transform: Transform::from_translation(Vec3::new(pos, 300.0, 1.0)),
                    ..default()
                },
                tracker: CollisionTracker::new(),
                ..Default::default()
            });
        } else {                    //75% chance for a raindrop
            commands.spawn(FallingThing {
                sprite: SpriteBundle {
                    texture: sprites.droplet.clone(),
                    transform: Transform::from_translation(Vec3::new(pos, 300.0, 1.0)),
                    ..default()
                },
                ..Default::default()
            });
        }
    }
}

fn move_objects(time: Res<Time>, mut objects: Query<(&mut Transform, &Speed)>) {
    for (mut position, speed) in objects.iter_mut() {
        position.translation.y += speed.vertical * time.delta_seconds();
        position.translation.x = (position.translation.x + speed.horizontal * time.delta_seconds()).clamp(-370.0, 370.0);
    }
}

fn process_falling_things(
    mut commands: Commands,
    mut objects: Query<(Entity, &DropletType, &Transform, &mut CollisionTracker)>,
    mut bucket: Query<(&Transform, &mut Speed), With<Bucket>>,
    mut water: Query<(&Transform, &mut Sprite, &mut WaterLevel)>,
    mut stats: ResMut<GameStats>,
    sfx: Res<Sfx>,
    audio: Res<Audio>,
    time: Res<Time>
) {
    let (bucket_pos, mut bucket_speed) = bucket.single_mut();
    for (e, droplet_type, drop_pos, mut tracker) in objects.iter_mut() {
        //tick the timers for the things that have one
        let (timer_option, force) = tracker.get_fields();
        if let Some(timer) = timer_option {
            timer.tick(time.delta());
            //it's been long enough, so we reset speed and give permission to tink again if the criteria is met
            if timer.finished() {
                timer.pause();
                timer.reset();
                bucket_speed.horizontal -= *force;
            }
        }
        //let things fall out of the world and unload them
        if drop_pos.translation.y < -272.0 {
            match droplet_type {
                DropletType::Raindrop => {
                    let mut iterator = water.iter_mut();
                    match iterator.next() {
                        None => {
                            warn!("No water for the raindrop to land in!? Spawning a WaterBundle...");
                            let mut new_water = WaterBundle::default();
                            new_water.water_level.add_water();
                            commands.spawn(new_water);
                        }
                        Some(start) => {
                            //find the closest pool of water to the droplet
                            let (mut closest_pos, mut closest_sprite, mut closest_water) = start;
                            while let Some(next) = iterator.next() {
                                let (water_pos, water_sprite, water_level) = next;
                                if (water_pos.translation.x - drop_pos.translation.x).abs() < (closest_pos.translation.x - drop_pos.translation.x).abs() {
                                    closest_pos = water_pos;
                                    closest_sprite = water_sprite;
                                    closest_water = water_level;
                                }
                            }
                            closest_water.add_water();
                            match closest_sprite.custom_size {
                                None => {
                                    warn!("No size for the water? Creating a new vector...");
                                    closest_sprite.custom_size = Some(Vec2::new(800.0, (2*closest_water.current) as f32));
                                }
                                Some(mut vector) => {
                                    //since the rectangle is centered, we make it twice as big instead of moving it up. half goes below the screen.
                                    vector.y = (2*closest_water.current) as f32;
                                    closest_sprite.custom_size = Some(vector);
                                }
                            }
                        }
                    }
                }
                DropletType::Sponge => {
                    let mut iterator = water.iter_mut();
                    match iterator.next() {
                        None => {
                            warn!("No water for the sponge to land in!? Spawning a WaterBundle...");
                            let new_water = WaterBundle::default();
                            commands.spawn(new_water);
                        }
                        Some(start) => {
                            //find the closest pool of water to the sponge
                            let (mut closest_pos, mut closest_sprite, mut closest_water) = start;
                            while let Some(next) = iterator.next() {
                                let (water_pos, water_sprite, water_level) = next;
                                if (water_pos.translation.x - drop_pos.translation.x).abs() < (closest_pos.translation.x - drop_pos.translation.x).abs() {
                                    closest_pos = water_pos;
                                    closest_sprite = water_sprite;
                                    closest_water = water_level;
                                }
                            }
                            closest_water.remove_water();
                            match closest_sprite.custom_size {
                                None => {
                                    warn!("No size for the water? Creating a new vector...");
                                    closest_sprite.custom_size = Some(Vec2::new(800.0, (2*closest_water.current) as f32));
                                }
                                Some(mut vector) => {
                                    vector.y = (2*closest_water.current) as f32;
                                    closest_sprite.custom_size = Some(vector);
                                }
                            }
                        }
                    }
                }
                DropletType::Stone => {
                    let mut rng = thread_rng();
                    match sfx.splash.choose(&mut rng) {
                        None => { warn!("Could not choose a stone splash sound effect."); }
                        Some(sound) => { audio.play(sound.clone()); }
                    }
                    //if we despawn a rock before its collision timer finishes, we want to reset the bucket's speed.
                    if let Some(timer) = timer_option {
                        if !timer.paused() {
                            bucket_speed.horizontal -= *force;
                        }
                    }
                }
            }
            commands.entity(e).despawn();
        }
        //check if things overlap with the bucket and behave appropriately
        else if distance_between(drop_pos, bucket_pos) < 64.0 {
            match droplet_type {
                DropletType::Raindrop => {
                    stats.score += 1;
                    commands.entity(e).despawn();
                    let mut rng = thread_rng();
                    match sfx.drop.choose(&mut rng) {
                        None => { warn!("Could not choose a droplet splash sound effect."); }
                        Some(sound) => { audio.play(sound.clone()); }
                    }
                }
                DropletType::Sponge => {
                    stats.score -= 1;
                    commands.entity(e).despawn();
                }
                DropletType::Stone => {
                    if let Some(timer) = timer_option {
                        //this timer just began, so let's play a tink and modify speed
                        if timer.paused() {
                            timer.unpause();
                            if drop_pos.translation.x > bucket_pos.translation.x {
                                *force = -600.0;
                            }
                            else {
                                *force = 600.0;
                            }
                            bucket_speed.horizontal += *force;

                            let mut rng = thread_rng();
                            match sfx.tink.choose(&mut rng) {
                                None => { warn!("Could not choose a stone tink sound effect."); }
                                Some(sound) => { audio.play(sound.clone()); }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn distance_between(a: &Transform, b: &Transform) -> f32 {
    ((a.translation.x - b.translation.x).powi(2) + (a.translation.y - b.translation.y).powi(2)).sqrt()
}


//      INPUT PROCESSING      \\
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum BucketAction {
    // This is the list of "things in the game I want to be able to do based on input"
    Left,
    Right
}
fn move_bucket(mut query: Query<(&ActionState<BucketAction>, &mut Speed), With<Bucket>>) {
    let (action_state, mut speed) = query.single_mut();
    // Each action has a button-like state of its own that you can check
    if action_state.just_pressed(BucketAction::Left) {
        speed.horizontal -= 300.0;
    }
    else if action_state.just_released(BucketAction::Left) {
        speed.horizontal += 300.0;
    }
    if action_state.just_pressed(BucketAction::Right) {
        speed.horizontal += 300.0;
    }
    else if action_state.just_released(BucketAction::Right) {
        speed.horizontal -= 300.0;
    }
}
