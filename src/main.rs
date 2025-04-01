use nannou::image::GenericImage;
use nannou::image::{self, GenericImageView};
use nannou::prelude::*;
use nannou::prelude::{wgpu, App, Frame, Key, LoopMode, MousePressed, Update, WindowEvent};

use std::{cell, env, thread, time};

static PAD_HEIGHT_FACTOR: f32 = 0.1;

// Build a solved board with numbers up to height * width - 1
fn solved_board(size: usize) -> Vec<Vec<usize>> {
    let mut board = vec![vec![0; size]; size];
    for row in 0..size {
        for col in 0..size {
            board[row][col] = (size - row - 1) * size + col + 1;
        }
    }
    board[0][size - 1] = 0;
    board
}

struct Model {
    grid_size: usize,
    flag_scramble: bool,
    scramble_count: usize,
    board: Vec<Vec<usize>>,
    image_original: image::DynamicImage,
    image_solved: image::DynamicImage,
    image: image::DynamicImage,
    texture: wgpu::Texture,
}

impl Model {
    /// Reset board
    fn reset(&mut self) {
        self.board = solved_board(self.grid_size);
    }

    /// Returns the indices of the empty space.
    fn index_empty(&self) -> (usize, usize) {
        let iy = self.board.iter().position(|r| r.contains(&0)).unwrap();
        let ix = self.board[iy].iter().position(|&x| x == 0).unwrap();

        (ix, iy)
    }

    /// When the user clicks on a piece, this function checks
    /// if that piece can be moved and returns `true` if the piece
    // can be moved, and `false` otherwise.
    fn is_move_valid(&self, ix: usize, iy: usize) -> bool {
        let (empty_x, empty_y) = self.index_empty();

        ix.abs_diff(empty_x) + iy.abs_diff(empty_y) == 1
    }

    /// Move the piece at `(ix, iy)` to the empty space.
    /// Check if the move is valid.
    fn try_move(&mut self, ix: usize, iy: usize) {
        //println!("Trying to move piece at index {ix}, {iy}");
        match self.is_move_valid(ix, iy) {
            true => {
                //println!("Move is valid");
                let (empty_x, empty_y) = self.index_empty();
                self.board[empty_y][empty_x] = self.board[iy][ix];
                self.board[iy][ix] = 0;
            }
            false => {
                //println!("Move is invalid");
                ()
            }
        }
    }

    /// Scramble the puzzle by randomly clicking everywhere
    fn scramble(&mut self) {
        loop {
            let ix = random_range(0, self.grid_size);
            let iy = random_range(0, self.grid_size);
            if self.is_move_valid(ix, iy) {
                self.try_move(ix, iy);
                return;
            }
        }
    }
    /// Update the image to show the current state of the board
    fn update_image(&mut self) {
        let (size, _h) = self.image_solved.dimensions();
        let cell_size = size as usize / self.grid_size;

        // Create a new image with the same size as the board
        let mut new_image = image::DynamicImage::new_rgba8(size, size);

        // Draw the pieces on the new image
        for row in 0..self.grid_size {
            for col in 0..self.grid_size {
                let piece = self.board[row][col];
                let other: image::DynamicImage;
                let x = (col * cell_size) as u32;
                let y = size - ((row + 1) * cell_size) as u32;
                if piece != 0 {
                    other = self.image_solved.crop_imm(
                        ((piece - 1) % self.grid_size) as u32 * cell_size as u32,
                        (piece / self.grid_size) as u32 * cell_size as u32,
                        cell_size as u32,
                        cell_size as u32,
                    );
                } else {
                    // If the piece is 0, we want to draw a black square
                    other = image::DynamicImage::new_rgba8(cell_size as u32, cell_size as u32);
                }
                new_image
                    .copy_from(&other, x, y)
                    .expect("Failed copying image");
            }
        }

        self.image = new_image;
    }
}

fn main() {
    nannou::app(model)
        .update(update)
        .loop_mode(LoopMode::Wait)
        .run();
}

fn model(app: &App) -> Model {
    let args: Vec<_> = env::args().collect();

    let grid_size = match args.len() {
        2 => {
            let size = args[1].parse().unwrap();
            size
        }
        _ => 4,
    };

    let _window = app
        .new_window()
        .size(300, 300)
        .title("Sliding Puzzle")
        .view(view)
        .event(event)
        .resized(window_resized)
        .build()
        .unwrap();

    let pad = (app.window_rect().h() * PAD_HEIGHT_FACTOR) as u32;
    let img_size = 300 - 2 * pad;

    let image_original = image::open("image.png").unwrap();

    let image_solved =
        image_original.resize_to_fill(img_size, img_size, image::imageops::FilterType::Nearest);
    let image = image_solved.clone();
    let texture = wgpu::Texture::from_image(app, &image);

    Model {
        grid_size,
        flag_scramble: false,
        scramble_count: 0,
        board: solved_board(grid_size),
        image_original,
        image_solved,
        image,
        texture,
    }
}

fn window_resized(_app: &App, model: &mut Model, dim: Vec2) {
    let pad = (dim.y * PAD_HEIGHT_FACTOR) as u32;
    let img_size = dim.y.min(dim.x) as u32 - 2 * pad;
    model.image_solved = model.image_original.resize_to_fill(
        img_size,
        img_size,
        image::imageops::FilterType::Nearest,
    );
}

fn update(app: &App, model: &mut Model, _update: Update) {
    if model.flag_scramble {
        model.scramble();
        thread::sleep(time::Duration::from_millis(15));
        model.scramble_count += 1;
        if model.scramble_count > 100 {
            model.scramble_count = 0;
            model.flag_scramble = false;
            app.set_loop_mode(LoopMode::Wait);
        }
    }
    model.update_image();
    model.texture = wgpu::Texture::from_image(app, &model.image);
}

fn event(app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        MousePressed(_button) => {
            // Check if the user clicked on an arrow
            // and move it if it can be moved.
            let win = app.window_rect();
            let pad = win.h() * PAD_HEIGHT_FACTOR;
            let cell_size = (win.h().min(win.w()) - 2.0 * pad) / model.grid_size as f32;
            let board_size = cell_size * model.grid_size as f32;
            if app.mouse.x.abs().max(app.mouse.y.abs()) > board_size / 2.0 {
                println!("Clicked outside the board");
                return;
            }
            let x_offset = (win.w() - 2.0 * pad - board_size) / 2.0;
            let y_offset = (win.h() - 2.0 * pad - board_size) / 2.0;

            let ix_clicked = (model.grid_size as f32
                * (app.mouse.x + win.w() / 2.0 - pad - x_offset)
                / (win.w() - 2.0 * pad - 2.0 * x_offset)) as usize;
            let iy_clicked = (model.grid_size as f32
                * (app.mouse.y + win.h() / 2.0 - pad - 2.0 * y_offset)
                / (win.h() - 2.0 * pad - y_offset)) as usize;
            println!("Indices clicked: {}, {}", ix_clicked, iy_clicked);
            model.try_move(ix_clicked, iy_clicked);
        }
        KeyPressed(Key::R) => model.reset(),
        KeyPressed(Key::S) => {
            app.set_loop_mode(LoopMode::RefreshSync);
            model.flag_scramble = true;
        }
        _ => (),
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    draw.texture(&model.texture).x_y(0.0, 0.0);

    // draw the board
    let win = app.window_rect();
    let pad = win.h() * PAD_HEIGHT_FACTOR;
    let cell_size = (win.w().min(win.h()) - 2.0 * pad) / model.grid_size as f32;

    let font_size = (cell_size / 2.0) as u32;

    let x_offset = (win.w() - 2.0 * pad - cell_size * model.grid_size as f32) / 2.0;
    let y_offset = (win.h() - 2.0 * pad - cell_size * model.grid_size as f32) / 2.0;

    // draw all the cells
    for row in 0..model.grid_size {
        let y = win.bottom() + y_offset + pad + row as f32 * cell_size + cell_size / 2.0;

        for col in 0..model.grid_size {
            let x = win.left() + x_offset + pad + col as f32 * cell_size + cell_size / 2.0;

            let piece = model.board[row][col];

            // draw the cell
            draw.rect()
                .x_y(x, y)
                .w_h(cell_size, cell_size)
                .no_fill()
                .stroke(GREY)
                .stroke_weight(2.0);

            // draw the number of the piece

            let text = match piece {
                0 => String::from(""),
                _ => piece.to_string(),
            };

            let text_area = geom::Rect::from_w_h(cell_size, cell_size).relative_to([-x, -y]);

            draw.text(&text)
                .font_size(font_size)
                .xy(text_area.xy())
                .wh(text_area.wh())
                .align_text_middle_y()
                .center_justify()
                .color(BLACK);
        }
    }

    draw.to_frame(app, &frame).unwrap();
}
