use std::fs::File;
use std::io::{BufRead, BufReader};
use bevy::prelude::*;
use bevy::time::{FixedTimestep, Stopwatch};
use bevy::input::keyboard::KeyboardInput;
use bevy_atmosphere::prelude::*;
use bevy_rapier3d::prelude::*;
use lerp::Lerp;
use rand::Rng;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    GameStart,
    Playing,
    GameOver,
}

struct Score {
    best:f32,
    next:String,
    time:Stopwatch
}

impl Default for Score{
    fn default() -> Self {
        Self {
            best:10000.0,
            next:"2".to_string(),
            time:Stopwatch::new()
        }
    }
}

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct SimpleFly{
    yaw:f32,
    pitch:f32,
    current_horizontal:f32,
    current_vertical:f32,
    follow:f32
}
impl Default for SimpleFly {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            current_horizontal: 0.0,
            current_vertical: 0.0,
            follow: 1.6
        }
    }
}

#[derive(Component)]
struct Laser;

#[derive(Component)]
struct Card{
    text:String
}

impl Card {
    fn get_score(&self)->String {
        let first = self.text.rfind("_").unwrap();
        let mut text = self.text[first+1..].to_string();
        if text.starts_with("0") {
            text = text[1..].to_string();
        }
        text
    }

}

#[derive(Component)]
struct Besttext;

#[derive(Component)]
struct Starttext;

#[derive(Component)]
struct Timetext;

#[derive(Component)]
struct NextCardtext;

#[derive(Component)]
struct EffectTime {
    timer: Timer
}

struct CreateEffectEvent(Vec3);

const GAMEOVER:usize =  13;
struct Stack {
    cards: Vec<String>,
    texts: Vec<String>,
    current:usize
}

impl Default for Stack {
    fn default() -> Self {
        Self{
            cards: vec!["2".to_string(),"3".to_string(),"4".to_string(),"5".to_string(),"6".to_string(),
                        "7".to_string(),"8".to_string(),"9".to_string(),"10".to_string(),"J".to_string(),
                        "Q".to_string(),"K".to_string(),"A".to_string()],
            current: 0,
            texts: vec![]
        }
    }
}

struct Talon {
    cards: Vec<String>
}

impl Default for Talon {
    fn default() -> Self {
        Self::new("assets/cards/_cards.csv".to_string())
    }
}

impl Talon {
    fn new(file_name: String) -> Self {
        let f = File::open(file_name).unwrap();
        let reader = BufReader::new(f);
        let mut l: Vec<String> = vec![];
        for line in reader.lines() {
            l.push(line.unwrap())
        };
        Self {
            cards:l
        }
    }
}

struct CountLaser{
    value:i32
}

struct NextCardPosition{
    position:Vec3
}

const FIRST_CARD_POSITION: Vec3 = Vec3::new(-15.0, -12.5,-28.0);

const SHIP_POSTION: Vec3 = Vec3::new(0.0, -1.0, -8.0);

fn main() {
    App::new()
        //add config resources
        .insert_resource(Msaa {samples: 4})
        .insert_resource(WindowDescriptor{
            title: "bevy air ace".to_string(),
            width: 800.0,
            height: 600.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
        .insert_resource(Score::default())
        .add_event::<CreateEffectEvent>()
        //bevy itself
        .add_plugins(DefaultPlugins)
        .add_plugin(AtmospherePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_camera)
        .add_state(GameState::GameStart)
        .add_system_set(
            SystemSet::on_enter(GameState::GameStart)
                .with_system(setup)
        )
        .add_system_set(
            SystemSet::on_update(GameState::GameStart)
                .with_system(any_key_pressed)
        )
        .add_system_set(
            SystemSet::on_exit(GameState::GameStart)
                .with_system(exit_start)
        )
        .add_system_set(
            SystemSet::on_enter(GameState::Playing)
                .with_system(setup_playing)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                        .with_system(fly_simple)
                        .with_system(spawn_laser)
                        .with_system(moving)
                        .with_system(create_effect)
                        .with_system(remove_effect)
                        .with_system(collision)
                        .with_system(scoreboard)
                        .with_system(despawn_card)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_run_criteria(FixedTimestep::step(2.0))
                .with_system(spawn_card)
        )
        .add_system_set(
            SystemSet::on_enter(GameState::GameOver)
                .with_system(setup_gameover)
        )
        .add_system_set(
            SystemSet::on_update(GameState::GameOver)
                .with_system(any_key_pressed_gameover)
        )
        .add_system_set(
            SystemSet::on_exit(GameState::GameOver)
                .with_system(exit_gameover)
        )
        .run();
}

fn setup_camera(
    mut commands: Commands
) {
    commands.
        spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.0, 0.0).looking_at(SHIP_POSTION.clone()+Vec3::new(0.0,1.0,0.0), Vec3::Y),
            ..Default::default()
        })
        .insert(AtmosphereCamera(None))
        .insert(UiCameraConfig {
            show_ui: true,
            ..default()
        })
        .insert(Camera{});
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(CountLaser{value:0});
    commands.insert_resource(Talon::default());
    commands.insert_resource(NextCardPosition{position:FIRST_CARD_POSITION});
    commands.insert_resource(Stack::default());

    //light
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 4.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });

    //ship

    commands.spawn_bundle(SceneBundle {
        scene: asset_server.load("models/ship1.glb#Scene0"),
        transform:Transform::from_translation(SHIP_POSTION.clone()),
        ..Default::default()
    })
        .insert(Ship{})
        .insert(SimpleFly::default());

    // Start
    commands.spawn_bundle(TextBundle {
        text: Text::from_section(
            " Bevy Air Ace \n \
                    \n \
                    by sabi@nelson-games.de \n \
                    \n \
                    control ship with arrow keys \n \
                    fire with space \n \
                    \n \
                    press any key to start",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 48.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            }
        ),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(15.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Starttext);
}


fn setup_playing(
    mut commands: Commands,
    mut score: ResMut<Score>,
    asset_server: Res<AssetServer>,
) {

    score.time.reset();

    // scoreboard
    commands.spawn_bundle(TextBundle {
        text: Text::from_section(
            "Best:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            }
        ),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Besttext);

    commands.spawn_bundle(TextBundle {
        text: Text::from_section(
            "Next Card:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            }
        ),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                right: Val::Px(350.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(NextCardtext);

    commands.spawn_bundle(TextBundle {
        text: Text::from_section(
            "Time:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            }
        ),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                right: Val::Px(25.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Timetext);
}


fn fly_simple(
    time:Res<Time>,
    keyboard_input:Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut SimpleFly)>
)
{
    let  yaw_amount = 1.0;
    let  pitch_amount = 1.0;

    for (mut transform,mut simple_fly) in query.iter_mut() {
        let mut horizontal = if keyboard_input.pressed(KeyCode::Left) {
            1.
        } else if keyboard_input.pressed(KeyCode::Right) {
            -1.
        } else {
            0.0
        };
        let mut vertical:f32 = if keyboard_input.pressed(KeyCode::Down) {
            -1.
        } else if keyboard_input.pressed(KeyCode::Up) {
            1.
        } else {
            0.0
        };

        horizontal = simple_fly.current_horizontal.lerp(horizontal,simple_fly.follow*time.delta_seconds());
        vertical = simple_fly.current_vertical.lerp(vertical,simple_fly.follow*time.delta_seconds());

        simple_fly.yaw += horizontal * yaw_amount * time.delta_seconds();
        simple_fly.yaw = simple_fly.yaw.clamp(-0.9,0.9);
        simple_fly.pitch += vertical * pitch_amount * time.delta_seconds();
        simple_fly.pitch = simple_fly.pitch.clamp(-0.9,0.9);

        transform.rotation = Quat::from_euler( EulerRot::YXZ,
                                               simple_fly.yaw,
                                               simple_fly.pitch,
                                               simple_fly.yaw);

        simple_fly.current_horizontal = horizontal;
        simple_fly.current_vertical = vertical;
    }
}

const MAX_LASER:i32=10;

fn spawn_laser(
    mut count_laser: ResMut<CountLaser>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard_input:Res<Input<KeyCode>>,
    query: Query<&Transform, With<Ship>>
)
{
    if keyboard_input.just_pressed(KeyCode::Space) {
        let ship_transform = query.single();
        if count_laser.value <= MAX_LASER {
            count_laser.value += 1;
            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::new(0.1, 0.1, 1.6))),
                material: materials.add(StandardMaterial {
                    base_color: Color::LIME_GREEN,
                    emissive: Color::LIME_GREEN,
                    ..Default::default()
                }),
                transform: Transform {
                    translation: ship_transform.translation,
                    rotation: ship_transform.rotation,
                    scale: Vec3::new(1.0, 1.0, 1.0)
                },
                ..Default::default()
            })
                //.insert(Speed { value: 10.0 })
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Sleeping::disabled())
                .insert(Collider::cuboid(0.1/2.0,
                                         0.1/2.0,
                                         1.6/2.0))
                .insert(Velocity {
                    linvel: ship_transform.forward()*48.0,
                    ..Default::default()
                })
                .insert(Laser);
        }
    }
}

const MAX_DISTANCE:f32 = 50.0;

fn moving(
    mut commands: Commands,
    mut count_laser: ResMut<CountLaser>,
    mut query: Query<(Entity, &mut Transform), With<Laser>>,
    query_ship: Query<&Transform, (With<Ship>, Without<Laser>)>
){
    let ship_transform = query_ship.single();
    for (entity, transform) in query.iter_mut() {
        if ship_transform.translation.distance(transform.translation) > MAX_DISTANCE {
            commands.entity(entity).despawn_recursive();
            count_laser.value -= 1;
        }
    }
}

const EFFECT_SIZE:f32=0.1;
const EFFECT_TIME:f32=2.0;

fn create_effect(
    mut commands: Commands,
    mut event_create_effect: EventReader<CreateEffectEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
)
{
    let mut rng = rand::thread_rng();
    for event in event_create_effect.iter() {
        let pos = event.0;
        for x in -2..2 {
            for y in 0..2 {
                for z in -2..2 {
                    commands
                        .spawn_bundle(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Box::new(0.1, 0.1, 0.1))),
                            material: materials.add(StandardMaterial {
                                metallic: 0.5,
                                emissive: Color::rgb(1.0, 0.5, 0.0),
                                ..Default::default()
                            }),
                            transform: Transform {
                                translation: Vec3::new(x as f32 * EFFECT_SIZE+pos.x,
                                                       y as f32 * EFFECT_SIZE+pos.y,
                                                       z as f32 * EFFECT_SIZE+pos.z),
                                rotation: Quat::from_rotation_x(0.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(RigidBody::Dynamic)
                        .insert(ExternalImpulse {
                            impulse: Vec3::new(rng.gen_range(-0.01..0.01),
                                               0.01,
                                               rng.gen_range(-0.01..0.01)),
                            torque_impulse: Vec3::new(0.0, 0.0, 0.0),
                        })
                        .insert(EffectTime{
                            timer: Timer::from_seconds(EFFECT_TIME,false)
                        })
                        .insert(Sleeping::disabled())
                        .insert(Collider::cuboid(0.1 / 2.0, 0.1 / 2.0, 0.1 / 2.0));
                }
            }
        }
    }
}

fn remove_effect(
    mut commands: Commands,
    time:Res<Time>,
    mut query: Query<(Entity, &mut EffectTime)>
)
{
    for (entity, mut timer) in query.iter_mut() {
        timer.timer.tick(time.delta());
        if timer.timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn collision(
    mut collision_events: EventReader<CollisionEvent>,
    mut state: ResMut<State<GameState>>,
    mut next_card_position:ResMut<NextCardPosition>,
    mut talon:ResMut<Talon>,
    mut stack: ResMut<Stack>,
    mut count_laser: ResMut<CountLaser>,
    mut query_card: Query<(Entity,&mut Transform, &Card), Without<Laser>>,
    query_laser: Query<(Entity, &Transform), With<Laser>>,
    mut event_create_effect: EventWriter<CreateEffectEvent>,
    mut commands: Commands
){
    for e in collision_events.iter(){
        //println!("Collision");
        let mut remove_card_from_stack = "Nothing".to_string();
        for (entity_card, mut card_transform, card) in query_card.iter_mut() {
            match e {
                CollisionEvent::Started(e1, e2, _) => {
                    if e1 == &entity_card || e2 == &entity_card {
                        for (entity_laser, _) in query_laser.iter() {
                            if e1 == &entity_laser || e2 == &entity_laser {
                                event_create_effect.send(CreateEffectEvent(Vec3::from(card_transform.translation)));
                                commands.entity(entity_laser).despawn_recursive();
                                count_laser.value -= 1;
                                let score = card.get_score();
                                //println!("{}",score);
                                if score == stack.cards[stack.current]{
                                    stack.current +=1;
                                    stack.texts.push(card.text.clone());
                                    commands.entity(entity_card)
                                        .remove::<RigidBody>()
                                        .remove::<Collider>();
                                    card_transform.translation = next_card_position.position.clone();
                                    next_card_position.position.x += 1.0;
                                    if stack.current == GAMEOVER {
                                        state.set(GameState::GameOver).unwrap();
                                    }
                                } else {
                                    talon.cards.push(card.text.clone());
                                    commands.entity(entity_card).despawn_recursive();
                                    if stack.current > 0 {
                                        stack.current -=1;
                                        remove_card_from_stack = stack.texts[stack.current].clone();
                                        let c = stack.current;
                                        stack.texts.remove(c);
                                    }
                                }
                            }
                        }
                    }
                }
                CollisionEvent::Stopped(_, _, _) => {}
            }
        }
        if remove_card_from_stack != "Nothing".to_string() {
            for (entity_card, _, card) in query_card.iter_mut() {
                if  card.text == remove_card_from_stack {
                    commands.entity(entity_card).despawn_recursive();
                    talon.cards.push(remove_card_from_stack.clone())
                }
            }
        }
    }
}

const CARD_LIMIT_X: f32 = 8.0;

fn spawn_card(
    mut commands: Commands,
    mut talon:ResMut<Talon>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let card_id = rng.gen_range(0..talon.cards.len());

    //card
    let card_text = talon.cards[card_id].clone();
    let card_texture_handle = asset_server.load(format!("cards/{}.png",card_text).as_str());

    talon.cards.remove(card_id);

    let card_aspect = 1.0;

    let card_quad_width = 3.2;
    let card_quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        card_quad_width,
        card_quad_width * card_aspect,
    ))));

    let card_material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(card_texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..Default::default()
    });

    commands.spawn_bundle(PbrBundle {
        mesh: card_quad_handle.clone(),
        material: card_material_handle,
        transform: Transform {
            translation: SHIP_POSTION.clone()+Vec3::new(rng.gen_range(-CARD_LIMIT_X..CARD_LIMIT_X),
                                                        8.0,
                                                        -16.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(RigidBody::Dynamic)
        .insert(Sleeping::disabled())
        .insert(Collider::cuboid(card_quad_width/2.0-0.5,
                                 card_quad_width * card_aspect/2.0,
                                 0.4))
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(GravityScale(0.5))
        .insert(Card{text:card_text.clone()});
}

fn scoreboard(
    mut score: ResMut<Score>,
    stack: Res<Stack>,
    time:Res<Time>,
    mut best_query: Query<&mut Text, (With<Besttext>, Without<NextCardtext>,Without<Timetext>)>,
    mut next_query: Query<&mut Text, (With<NextCardtext>,Without<Timetext>,Without<Besttext>)>,
    mut time_query: Query<&mut Text, (With<Timetext>,Without<Besttext>,Without<NextCardtext>)>
) {
    let mut best_text = best_query.single_mut();
    best_text.sections[0].value = format!("Best: {:.1}", score.best);

    let mut next_text = next_query.single_mut();
    next_text.sections[0].value = format!("Next card: {}", stack.cards[stack.current]);

    let mut time_text = time_query.single_mut();
    score.time.tick(time.delta());
    time_text.sections[0].value = format!("Time: {:.1}", score.time.elapsed_secs());
}

const CARD_LIMIT_Y:f32=-20.0;

fn despawn_card(
    mut commands: Commands,
    mut talon:ResMut<Talon>,
    mut query_card: Query<(Entity,&mut Transform, &Card)>,
) {
    for (e, transform, card) in query_card.iter_mut(){
        if transform.translation.y <=CARD_LIMIT_Y {
            talon.cards.push(card.text.clone());
            commands.entity(e).despawn_recursive();
        }
    }
}

fn any_key_pressed(
    mut state: ResMut<State<GameState>>,
    mut key_event: EventReader<KeyboardInput>,
) {
    use bevy::input::ButtonState;

    for ev in key_event.iter() {
        match ev.state {
            ButtonState::Pressed => {
            }
            ButtonState::Released => {
                state.set(GameState::Playing).unwrap();
            }
        }
    }
}

fn exit_start(
    mut commands: Commands,
    mut query: Query<Entity, With<Starttext>>
) {
    let start_text = query.single_mut();
    commands.entity(start_text).despawn_recursive();
}

fn setup_gameover(
    mut commands: Commands,
    mut score: ResMut<Score>,
    asset_server: Res<AssetServer>,
) {
    let time = score.time.elapsed_secs();
    if time < score.best {
        score.best = time;
    }
    commands.spawn_bundle(TextBundle {
        text: Text::from_section(
            " Game over \n \
                    \n \
                    by sabi@nelson-games.de \n \
                    \n \
                    press any key to start",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 48.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            }
        ),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(15.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
}

fn any_key_pressed_gameover(
    mut state: ResMut<State<GameState>>,
    mut key_event: EventReader<KeyboardInput>,
) {
    use bevy::input::ButtonState;

    for ev in key_event.iter() {
        match ev.state {
            ButtonState::Pressed => {
            }
            ButtonState::Released => {
                state.set(GameState::GameStart).unwrap();
            }
        }
    }
}

fn exit_gameover(
    mut commands: Commands,
    entities: Query<Entity,Without<Camera>>
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<CountLaser>();
    commands.remove_resource::<Talon>();
    commands.remove_resource::<NextCardPosition>();
    commands.remove_resource::<Stack>();
}