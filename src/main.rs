use macroquad::prelude::*;

struct Shape {
    size: f32,
    speed: f32,
    x: f32,
    y: f32,
}

impl Shape {
    
}

#[macroquad::main("MyGame")]
async fn main() {
    //seed RNG and set some useful consts
    rand::srand(miniquad::date::now() as u64);
    const MOVEMENT_SPEED: f32 = 200.0;
    
    let mut squares = vec![];
    //set up the player shape
    let mut circle = Shape {
        size: 32.0,
        speed: MOVEMENT_SPEED,
        x: screen_width() / 2.0,
        y: screen_height() / 2.0,
    };

    loop {
        clear_background(DARKPURPLE);
        
        //get player input
        let delta_time = get_frame_time();
        if is_key_down(KeyCode::Right) {
            circle.x+= circle.speed * delta_time;
        }
        if is_key_down(KeyCode::Left) {
            circle.x-= circle.speed * delta_time;
        }
        if is_key_down(KeyCode::Down) {
            circle.y+= circle.speed * delta_time;
        }
         if is_key_down(KeyCode::Up) {
            circle.y-= circle.speed * delta_time;
        }
        
        // prevent player from moving off screen
        circle.x = clamp(circle.x, circle.size / 2.0, screen_width()-(circle.size / 2.0));
        circle.y = clamp(circle.y, circle.size / 2.0, screen_height()-(circle.size / 2.0));
        
        //create randomly sized squares
        if rand::gen_range(0, 99) >= 95 {        
            let size = rand::gen_range(16.0, 64.0);
            squares.push(Shape {
                size,
                speed: rand::gen_range(50.0, 150.0),
                x: rand::gen_range(size / 2.0, screen_width() - size / 2.0),
                y: -size,
            });
        }

        //move the squares
        for square in &mut squares {
            square.y += square.speed * delta_time;
        }

        //remove squares if they have left the screen
        squares.retain(|square| square.y < screen_height() +square.size);

        //draw everything
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


        next_frame().await
    }
}
