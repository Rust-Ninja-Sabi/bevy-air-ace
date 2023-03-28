use std::time::Duration;
use bevy::prelude::*;
use bevy::window::{WindowResolution,PresentMode};
use bevy::time::Stopwatch;
use bevy::time::common_conditions::on_timer;
use bevy::input::keyboard::KeyboardInput;
use bevy::pbr::NotShadowReceiver;
use bevy_rapier3d::prelude::*;
use bevy_mod_raycast::{RaycastMesh,RaycastSystem,DefaultRaycastingPlugin,DefaultPluginState,
                       RaycastSource, RaycastMethod};
use rand::Rng;

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    GameStart,
    Playing,
    GameOver,
}

#[derive(Clone, Reflect)]
struct MyRaycastSet;

#[derive(Resource)]
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

struct CreateLaserEvent;

const GAMEOVER:usize =  13;

#[derive(Resource)]
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

const ALL_CARDS:&str = "card_hearts_A,card_hearts_02,card_hearts_03,card_hearts_04,card_hearts_05,card_hearts_06,card_hearts_07,card_hearts_08,
card_hearts_09,\
card_hearts_10,\
card_hearts_J,\
card_hearts_Q,\
card_hearts_K,\
card_diamonds_A,\
card_diamonds_02,\
card_diamonds_03,\
card_diamonds_04,\
card_diamonds_05,\
card_diamonds_06,\
card_diamonds_07,\
card_diamonds_08,\
card_diamonds_09,\
card_diamonds_10,\
card_diamonds_J,\
card_diamonds_Q,\
card_diamonds_K,\
card_clubs_A,\
card_clubs_02,\
card_clubs_03,\
card_clubs_04,\
card_clubs_05,\
card_clubs_06,\
card_clubs_07,\
card_clubs_08,\
card_clubs_09,\
card_clubs_10,\
card_clubs_J,\
card_clubs_Q,\
card_clubs_K,\
card_spades_A,\
card_spades_02,\
card_spades_03,\
card_spades_04,\
card_spades_05,\
card_spades_06,\
card_spades_07,\
card_spades_08,\
card_spades_09,\
card_spades_10,\
card_spades_J,\
card_spades_Q,\
card_spades_K";

#[derive(Resource)]
struct Talon {
    cards: Vec<String>
}

impl Default for Talon {
    fn default() -> Self {
        Self::new()
    }
}

impl Talon {
    fn new() -> Self {
        let mut l:Vec<String>=vec![];
        for s in ALL_CARDS.split(","){
             l.push(s.to_string());
        }
        Self {
            cards:l
        }
    }
}

#[derive(Resource)]
struct CountLaser{
    value:i32
}

#[derive(Resource)]
struct NextCardPosition{
    position:Vec3
}

const FIRST_CARD_POSITION: Vec3 = Vec3::new(-15.0, -12.5,-28.0);

const SHIP_POSTION: Vec3 = Vec3::new(0.0, -1.0, -8.0);

fn main() {
    App::new()
        //add config resources
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
        .insert_resource(Score::default())
        .add_event::<CreateEffectEvent>()
        .add_event::<CreateLaserEvent>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy air ace".to_string(),
                present_mode: PresentMode::AutoNoVsync, // Reduces input lag.
                resolution: WindowResolution::new(920.0, 640.0),
                ..default()
            }),
            ..default()
        }))
        //.add_plugin(AtmospherePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .add_startup_system(setup_camera)
        .add_system(
            update_raycast_with_cursor
                .in_base_set(CoreSet::First)
                .before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_state::<GameState>()
        .add_system(setup.in_schedule(OnEnter(GameState::GameStart)))
        .add_system(any_key_pressed.in_set(OnUpdate(GameState::GameStart)))
        .add_system(exit_start.in_schedule(OnExit(GameState::GameStart)))
        .add_system(setup_playing.in_schedule(OnEnter(GameState::Playing)))
        .add_systems((spawn_laser,
                      moving,
                      create_effect,
                      remove_effect,
                      collision,
                      scoreboard,
                      despawn_card,
                      mouse_button_input).in_set(OnUpdate(GameState::Playing)))
        .add_system(spawn_card
                        .in_set(OnUpdate(GameState::Playing))
                        .run_if(on_timer(Duration::from_secs_f32(2.0))))
        .add_system(setup_gameover.in_schedule(OnEnter(GameState::GameOver)))
        .add_system(any_key_pressed_gameover.in_set(OnUpdate(GameState::GameOver)))
        .add_system(exit_gameover.in_schedule(OnExit(GameState::GameOver)))
        .run();
}

fn setup_camera(
    mut commands: Commands
) {
    commands.insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor());

    commands.
        spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.0, 0.0).looking_at(SHIP_POSTION.clone()+Vec3::new(0.0,1.0,0.0), Vec3::Y),
            ..Default::default()
        })
        .insert(UiCameraConfig {
            show_ui: true,
            ..default()
        })
        .insert(RaycastSource::<MyRaycastSet>::new())
        .insert(Camera{});
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(CountLaser{value:0});
    commands.insert_resource(Talon::default());
    commands.insert_resource(NextCardPosition{position:FIRST_CARD_POSITION});
    commands.insert_resource(Stack::default());

    //light
    commands.spawn(DirectionalLightBundle {
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

    //whiteboard
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::new(40.0,32.0,1.0))),
            material: materials.add(Color::rgb(0.9, 0.9, 1.0).into()),
            transform: Transform::from_xyz(0.0,0.0,-30.0),
            ..Default::default()
        })
        .insert(NotShadowReceiver)
        .insert(RaycastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able

    //ship

    commands.spawn(SceneBundle {
        scene: asset_server.load("models/ship1.glb#Scene0"),
        transform:Transform::from_translation(SHIP_POSTION.clone()),
        ..Default::default()
    })
        .insert(Ship{});

    // Start
    commands.spawn(TextBundle {
        text: Text::from_section(
            " Bevy Air Ace \n \
                    \n \
                    by sabi@nelson-games.de \n \
                    \n \
                    control ship with mouse \n \
                    fire with click \n \
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
    commands.spawn(TextBundle {
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

    commands.spawn(TextBundle {
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

    commands.spawn(TextBundle {
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


const MAX_LASER:i32=10;

fn spawn_laser(
    mut event_create_laser: EventReader<CreateLaserEvent>,
    mut count_laser: ResMut<CountLaser>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&Transform, With<Ship>>
)
{
    for _ in event_create_laser.iter() {
        let ship_transform = query.single();
        if count_laser.value <= MAX_LASER {
            count_laser.value += 1;
            commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::new(0.1, 0.1, 1.6))),
                material: materials.add(StandardMaterial {
                    base_color: Color::LIME_GREEN,
                    emissive: Color::LIME_GREEN,
                    ..Default::default()
                }),
                transform: Transform {
                    translation: ship_transform.translation,
                    rotation: ship_transform.rotation.clone(),
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
                    linvel: ship_transform.forward()*84.0,
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
                        .spawn(PbrBundle {
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
                            timer: Timer::from_seconds(EFFECT_TIME,TimerMode::Once)
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
    mut state: ResMut<NextState<GameState>>,
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
                                        state.set(GameState::GameOver);
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

    commands.spawn(PbrBundle {
        mesh: card_quad_handle.clone(),
        material: card_material_handle,
        transform: Transform {
            translation: SHIP_POSTION.clone()+Vec3::new(rng.gen_range(-CARD_LIMIT_X..CARD_LIMIT_X),
                                                        10.0,
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
    mut game_state: ResMut<NextState<GameState>>,
    mut key_event: EventReader<KeyboardInput>,
) {
    use bevy::input::ButtonState;

    for ev in key_event.iter() {
        match ev.state {
            ButtonState::Pressed => {
            }
            ButtonState::Released => {
                game_state.set(GameState::Playing);
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
    commands.spawn(TextBundle {
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
    mut game_state: ResMut<NextState<GameState>>,
    mut key_event: EventReader<KeyboardInput>,
) {
    use bevy::input::ButtonState;

    for ev in key_event.iter() {
        match ev.state {
            ButtonState::Pressed => {
            }
            ButtonState::Released => {
                game_state.set(GameState::GameStart);
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

fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    // Grab the most recent cursor event if it exists:
    let cursor_position = match cursor.iter().last() {
        Some(cursor_moved) => cursor_moved.position,
        None => return,
    };

    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_position);
    }
}

fn mouse_button_input(
    buttons: Res<Input<MouseButton>>,
    mut event_create_laser: EventWriter<CreateLaserEvent>,
    mut query_ship: Query<&mut Transform, With<Ship>>,
    query: Query<&RaycastSource<MyRaycastSet>>
) {
    if buttons.just_released(MouseButton::Left) {
        for pick_source in query.iter() {
            let mut position = match pick_source.get_nearest_intersection() {
                Some((_, intersection)) => intersection.position(),
                None => return,
            };
            let mut ship_transform = query_ship.single_mut();
            let transform = ship_transform.looking_at(position, Vec3::Y);
            ship_transform.rotation = transform.rotation.clone();
            event_create_laser.send(CreateLaserEvent);
        }
    }
}