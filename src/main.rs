use macroquad::prelude::*;
use std::fs;
use macroquad_particles::{self as particles, AtlasConfig, ColorCurve, Emitter, EmitterConfig};
use macroquad::experimental::animation::{AnimatedSprite, Animation};
use macroquad::audio::{load_sound, play_sound, play_sound_once, set_sound_volume, PlaySoundParams};

const FRAGMENT_SHADER: &str = include_str!("starfield-shader.glsl");

const VERTEX_SHADER: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying float iTime;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec4 _Time;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    iTime = _Time.x;
}
";

struct Shape {
    size: f32,
    speed: f32,
    x: f32,
    y: f32,
    collided: bool,
}

impl Shape {
    fn collides_with(&self, other: &Self) -> bool {
        self.rect().overlaps(&other.rect())
    }

    fn rect(&self) -> Rect {
        Rect {
            x: self.x - self.size / 2.0,
            y: self.y - self.size / 2.0,
            w: self.size,
            h: self.size,
        }
    }
}

enum GameState {
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

fn particle_explosion() -> particles::EmitterConfig {
    particles::EmitterConfig {
        local_coords: false,
        one_shot: true,
        emitting: true,
        lifetime: 0.6,
        lifetime_randomness: 0.3,
        explosiveness: 0.65,
        initial_direction_spread: 2.0 * std::f32::consts::PI,
        //initial_velocity: 300.0,
        initial_velocity_randomness: 0.8,
        size: 16.0,
        size_randomness: 0.3,
        atlas: Some(AtlasConfig::new(5, 1, 0..)),
        ..Default::default()
    }
}

fn particle_exhaust() -> particles::EmitterConfig {
    particles::EmitterConfig {
        local_coords: false,
        one_shot: false,
        emitting: true,
        lifetime: 0.6,
        lifetime_randomness: 0.3,
        explosiveness: 0.4,
        initial_direction_spread: std::f32::consts::PI / 12.0,
        initial_velocity: -500.0,
        initial_velocity_randomness: 0.8,
        size: 3.0,
        size_randomness: 0.3,
        colors_curve: ColorCurve {
            start: DARKGRAY,
            mid: WHITE,
            end: BLACK,
        },
        ..Default::default()
    }
}

#[macroquad::main("MyGame")]
async fn main() {
    //seed RNG and set some useful consts
    rand::srand(miniquad::date::now() as u64);
    const MOVEMENT_SPEED: f32 = 200.0;
    
    //track enemies and bullets
    let mut squares = vec![];
    let mut bullets: Vec<Shape> = vec![];
    let mut explosions: Vec<(Emitter, Vec2)> = vec![];

    //set up the player shape
    let mut circle = Shape {
        size: 32.0,
        speed: MOVEMENT_SPEED,
        x: screen_width() / 2.0,
        y: screen_height() / 2.0,
        collided: false,
    };

    //create an exhaust stream for the player
    let mut exhaust = Emitter::new(EmitterConfig {
        amount: circle.size.round() as u32 * 2,
            ..particle_exhaust()
    });

    let mut game_state = GameState::MainMenu;
    let mut gitgud = false;
    let mut last_shot = get_time();
    let mut score: u32 = 0;
    let mut high_score: u32 = fs::read_to_string("highscore.dat")
        .map_or(Ok(0), |i| i.parse::<u32>())
        .unwrap_or(0);

    let mut direction_modifier: f32 = 0.0;
    let render_target = render_target(320, 150);
    render_target.texture.set_filter(FilterMode::Nearest);
    let material = load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment: FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("iResolution", UniformType::Float2),
                UniformDesc::new("direction_modifier", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    //import textures
    set_pc_assets_folder("assets");
    let ship_texture: Texture2D = load_texture("ship.png")
        .await
        .expect("Couldn't load file");
    ship_texture.set_filter(FilterMode::Nearest);
    let bullet_texture: Texture2D = load_texture("laser-bolts.png")
        .await
        .expect("Couldn't load file");
    bullet_texture.set_filter(FilterMode::Nearest);
    let explosion_texture: Texture2D = load_texture("explosion.png")
        .await
        .expect("Couldn't load file");
    explosion_texture.set_filter(FilterMode::Nearest);

    let enemy_small_texture: Texture2D = load_texture("enemy-small.png")
        .await
        .expect("Couldn't load file");
    enemy_small_texture.set_filter(FilterMode::Nearest);
    let enemy_medium_texture: Texture2D = load_texture("enemy-medium.png")
        .await
        .expect("Couldn't load file");
    enemy_medium_texture.set_filter(FilterMode::Nearest);
    let enemy_big_texture: Texture2D = load_texture("enemy-big.png")
        .await
        .expect("Couldn't load file");
    enemy_big_texture.set_filter(FilterMode::Nearest);
    build_textures_atlas();

    //music loading
    let theme_music = load_sound("8bit-spaceshooter.ogg").await.unwrap();
    let sound_explosion = load_sound("explosion.wav").await.unwrap();
    let sound_laser = load_sound("laser.wav").await.unwrap();

    //bullet sprite config
    let mut bullet_sprite = AnimatedSprite::new(
        16,
        16,
        &[
            Animation {
                name: "bullet".to_string(),
                row: 0,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "bolt".to_string(),
                row: 1,
                frames: 2,
                fps: 12,
            },
        ],
        true,
    );
    bullet_sprite.set_animation(1);

    //ship sprite config
    let mut ship_sprite = AnimatedSprite::new(
        16,
        24,
        &[
            Animation {
                name: "idle".to_string(),
                row: 0,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "slight_left".to_string(),
                row: 1,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "left".to_string(),
                row: 2,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "slight_right".to_string(),
                row: 3,
                frames: 2,
                fps: 12,
            },
            Animation {
                name: "right".to_string(),
                row: 4,
                frames: 2,
                fps: 12,
            },
        ],
        true,
        );


    //enemy sprite config
    let mut enemy_small_sprite = AnimatedSprite::new(
        17,
        16,
        &[Animation {
            name: "enemy_small".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );
    let mut enemy_medium_sprite = AnimatedSprite::new(
        32,
        16,
        &[Animation {
            name: "enemy_medium".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );
    let mut enemy_big_sprite = AnimatedSprite::new(
        32,
        32,
        &[Animation {
            name: "enemy_big".to_string(),
            row: 0,
            frames: 2,
            fps: 12,
        }],
        true,
    );


    //play music
    play_sound(
        &theme_music,
        PlaySoundParams {
            looped: true,
            volume: 0.2,
        },
    );

    loop {
        clear_background(BLACK);
        
        material.set_uniform("iResolution", (screen_width(), screen_height()));
        material.set_uniform("direction_modifier", direction_modifier);
        gl_use_material(&material);
        draw_texture_ex(
            &render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        gl_use_default_material();


        match game_state {
            GameState::MainMenu => {
                if is_key_pressed(KeyCode::Escape) {
                    std::process::exit(0);
                }

                if is_key_pressed(KeyCode::Space) {
                    squares.clear();
                    bullets.clear();
                    explosions.clear();
                    circle.x = screen_width() / 2.0;
                    circle.y = screen_height() / 2.0;
                    score = 0;
                    game_state = GameState::Playing;
                } 
                let title = "SHAPEWAR";
                let title_dimensions = measure_text(title, None, 150, 1.0);
                draw_text(
                    title,
                    screen_width() / 2.0 - title_dimensions.width / 2.0,
                    title_dimensions.height + 10.0,
                    150.0,
                    WHITE,
                );

                let text = "Press space";
                let text_dimensions = measure_text(text, None, 50, 1.0);
                draw_text(
                    text, 
                    screen_width() / 2.0 - text_dimensions.width / 2.0,
                    screen_height() / 2.0,
                    50.0,
                    WHITE,
                );
            },
            GameState::Playing => {         
                set_sound_volume(&theme_music, 0.8);
                
                //get player input
                ship_sprite.set_animation(0);
                let delta_time = get_frame_time();
                let frame_time = get_time();
                if is_key_down(KeyCode::Right) {
                    circle.x+= circle.speed * delta_time;
                    direction_modifier += 0.05 * delta_time;
                    ship_sprite.set_animation(4);
                }
                if is_key_down(KeyCode::Left) {
                    circle.x-= circle.speed * delta_time;
                    direction_modifier -= 0.05 * delta_time;
                    ship_sprite.set_animation(2);
                }
                if is_key_down(KeyCode::Down) {
                    circle.y+= circle.speed * delta_time;
                }
                 if is_key_down(KeyCode::Up) {
                    circle.y-= circle.speed * delta_time;
                }
                
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Paused;
                }

                // prevent player from moving off screen
                circle.x = clamp(circle.x, circle.size / 2.0, screen_width()-(circle.size / 2.0));
                circle.y = clamp(circle.y, circle.size / 2.0, screen_height()-(circle.size / 2.0));
                
                //shot
                if is_key_pressed(KeyCode::Space) && frame_time - last_shot > 0.5{ 
                    bullets.push(Shape {
                       x: circle.x,
                       y: circle.y -24.0,
                       speed: circle.speed * 2.0,
                       size: 32.0,
                       collided: false,
                    });
                    play_sound_once(&sound_laser);
                    last_shot = frame_time;
                }


                //create randomly sized squares
                if rand::gen_range(0, 99) >= 95 {        
                    let size = rand::gen_range(16.0, 64.0);
                    squares.push(Shape {
                        size,
                        speed: rand::gen_range(50.0, 150.0),
                        x: rand::gen_range(size / 2.0, screen_width() - size / 2.0),
                        y: -size,
                        collided: false,
                    });
                }

                //move non player objects
                for square in &mut squares {
                    square.y += square.speed * delta_time;
                }
                for bullet in &mut bullets {
                    bullet.y -= bullet.speed * delta_time;
                }

                ship_sprite.update();
                bullet_sprite.update();
                enemy_small_sprite.update();
                enemy_medium_sprite.update();
                enemy_big_sprite.update();


                //remove nonplayer objects if they have left the screen
                squares.retain(|square| square.y < screen_height() + square.size);
                bullets.retain(|bullet| bullet.y < screen_height() + bullet.size);

                //remove 'dead' objects
                squares.retain(|square| !square.collided);
                bullets.retain(|bullet| !bullet.collided);
                explosions.retain(|(explosion, _)| explosion.config.emitting);

                if squares.iter().any(|square| circle.collides_with(square)) {
                    if score == high_score {
                        fs::write("highscore.dat", high_score.to_string()).ok();
                        gitgud = true;
                    }
                    game_state = GameState::GameOver;
                }

                for square in squares.iter_mut() {
                    for bullet in bullets.iter_mut() {
                        if bullet.collides_with(square) {
                            bullet.collided = true;
                            square.collided = true;
                            score += square.size.round() as u32 / 2;
                            score += square.size.round() as u32 / 2; 
                            high_score = high_score.max(score);
                            explosions.push((
                                    Emitter::new(EmitterConfig {
                                        amount: square.size.round() as u32 * 2,
                                        initial_velocity: square.size * 2.0,
                                        texture: Some(explosion_texture.clone()),
                                        ..particle_explosion()
                                    }),
                                    vec2(square.x, square.y),
                                ));
                            play_sound_once(&sound_explosion);
                        }
                    }
                }


                //draw everything
                let circle_pos = vec2(circle.x, circle.y - (circle.size / 2.0));
                exhaust.draw(circle_pos);
                
                let ship_frame = ship_sprite.frame();
                draw_texture_ex(
                    &ship_texture,
                    circle.x - ship_frame.dest_size.x,
                    circle.y - ship_frame.dest_size.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(ship_frame.dest_size * 2.0),
                        source: Some(ship_frame.source_rect),
                        ..Default::default()
                    },
                );
                
                let enemy_small = enemy_small_sprite.frame();
                let enemy_medium = enemy_medium_sprite.frame();
                let enemy_big = enemy_big_sprite.frame();
                for square in &squares {
                    if square.size <= 32.0 {
                    draw_texture_ex(
                        &enemy_small_texture,
                        square.x - square.size / 2.0,
                        square.y - square.size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(square.size, square.size)),
                            source: Some(enemy_small.source_rect),
                            ..Default::default()
                        },
                    );
                    } else if square.size <=48.0 {
                    draw_texture_ex(
                        &enemy_medium_texture,
                        square.x - square.size / 2.0,
                        square.y - square.size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(square.size, square.size )),
                            source: Some(enemy_medium.source_rect),
                            ..Default::default()
                        },
                    );
                    } else {
                    draw_texture_ex(
                        &enemy_big_texture,
                        square.x - square.size / 2.0,
                        square.y - square.size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(square.size, square.size)),
                            source: Some(enemy_big.source_rect),
                            ..Default::default()
                        },
                    );
                    }


                }

                for (explosion, coords) in explosions.iter_mut() {
                    explosion.draw(*coords);
                }

                let bullet_frame = bullet_sprite.frame();
                for bullet in  &bullets {
                    draw_texture_ex(
                        &bullet_texture,
                        bullet.x - bullet.size / 2.0,
                        bullet.y -bullet.size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(bullet.size, bullet.size)),
                            source: Some(bullet_frame.source_rect),
                            ..Default::default()
                        },
                    );
                }

                draw_text(
                    format!("Score: {}", score).as_str(),
                    10.0,
                    35.0,
                    25.0,
                    WHITE,
                );
                let highscore_text = format!("High score: {}", high_score);
                let text_dimensions = measure_text(highscore_text.as_str(), None, 25, 1.0);
                draw_text(
                    highscore_text.as_str(),
                    screen_width() - text_dimensions.width -10.0,
                    35.0,
                    25.0,
                    WHITE,
                );
            },           
            GameState::Paused => {
                set_sound_volume(&theme_music, 0.0);
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Playing;
                }
                let text = "Paused";
                let text_dimensions = measure_text(text, None, 50, 1.0);
                

                draw_circle(circle.x, circle.y, circle.size / 2.0, YELLOW);
                
                for square in &squares {
                    draw_rectangle(
                        square.x - square.size / 2.0,
                        square.y - square.size / 2.0,
                        square.size,
                        square.size,
                        GREEN,
                    );
                }

                for bullet in  &bullets {
                    draw_circle(bullet.x, bullet.y, bullet.size / 2.0, RED);
                }

                draw_text(
                    text,
                    screen_width() / 2.0 - text_dimensions.width / 2.0,
                    screen_height() / 2.0,
                    50.0,
                    WHITE,
                );
            },       
            GameState::GameOver => {
                if is_key_pressed(KeyCode::Space) {
                    game_state = GameState::MainMenu;
                }
                let text = "GAME OVER!";
                let text_dimensions = measure_text(text, None, 50, 1.0);
                draw_text(
                    text,
                    screen_width() / 2.0 - text_dimensions.width / 2.0,
                    screen_height() / 2.0,
                    50.0,
                    RED,
                );
                if gitgud {
                    let text = "New High Score!";
                    let text_dimensions = measure_text(text, None, 30, 1.0);
                    draw_text(
                        text,
                        screen_width() / 2.0 - text_dimensions.width / 2.0,
                        screen_height() / 2.0 + text_dimensions.height,
                        30.0,
                        RED,
                    );
                }
            },
        }

        next_frame().await
    }
}
