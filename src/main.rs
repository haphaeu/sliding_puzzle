use nannou::image::GenericImage;
use nannou::image::{self, GenericImageView};
use nannou::prelude::*;
use nannou::prelude::{wgpu, App, Frame, Key, LoopMode, MousePressed, Update, WindowEvent};

use std::{env, fs, path::PathBuf, thread, time};

use env_logger::Builder;
use log::debug;

/// Initial window size, window is square.
/// User can resize to non-square size, in which
/// case the square grid will be centred in the window.
static START_WINDOW_SIZE: u32 = 300;

/// Padding around the grid is calculated as a factor
/// of the window height.
static PAD_HEIGHT_FACTOR: f32 = 0.1;

/// Build a solved board with numbers up to height * width - 1
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
    grid_size: usize,                    // Size of the square grid of the board
    flag_scramble: bool,                 // Flag to indicate if the board is being scrambled
    flag_show_numbers: bool,             // Flag to indicate if the numbers should be shown
    scramble_count: usize,               // Number of times the board has been scrambled
    board: Vec<Vec<usize>>,              // The board itself
    image_list: Vec<PathBuf>,            // List of images to use
    image_index_current: usize,          // Index of the current image
    image_original: image::DynamicImage, // Original image
    image_solved: image::DynamicImage,   // Resized image and cut square
    image: image::DynamicImage,          // Game display, ie, scrambled image
    texture: wgpu::Texture,              // Texture to display the image
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
        debug!("Trying to move piece at index {ix}, {iy}");
        match self.is_move_valid(ix, iy) {
            true => {
                debug!("Move is valid");
                let (empty_x, empty_y) = self.index_empty();
                self.board[empty_y][empty_x] = self.board[iy][ix];
                self.board[iy][ix] = 0;
            }
            false => {
                debug!("Move is invalid");
                ()
            }
        }
    }

    /// Randomly clicking everywhere until a valid move is found
    fn do_one_random_move(&mut self) {
        loop {
            let ix = random_range(0, self.grid_size);
            let iy = random_range(0, self.grid_size);
            if self.is_move_valid(ix, iy) {
                self.try_move(ix, iy);
                return;
            }
        }
    }
    /// Update the image to show the current state of the board,
    /// ie, cut the pieces from the solved image and paste them into the
    /// image shown in the board according to the current state of the board.
    fn update_image(&mut self) {
        let (size, _h) = self.image_solved.dimensions();
        let cell_size = size as usize / self.grid_size;

        // Create a new image with the same size as the board
        let mut new_image = image::DynamicImage::new_rgba8(size, size);

        // Draw the pieces on the new image
        for row in 0..self.grid_size {
            for col in 0..self.grid_size {
                let piece = self.board[row][col];
                if piece != 0 {
                    let x0 = ((piece - 1) % self.grid_size) as u32 * cell_size as u32;
                    let y0 = ((piece - 1) / self.grid_size) as u32 * cell_size as u32;
                    let little_square =
                        self.image_solved
                            .crop_imm(x0, y0, cell_size as u32, cell_size as u32);
                    let x = (col * cell_size) as u32;
                    let y = size - ((row + 1) * cell_size) as u32;
                    debug!("Row {row}, Col {col}, piece: {piece:2} at x0: {x0:3}, y0: {y0:3} into x: {x:3}, y: {y:3}");
                    new_image
                        .copy_from(&little_square, x, y)
                        .expect("Failed copying image");
                } else {
                    debug!("Row {row}, Col {col}, piece: {piece:2} - nothing to do");
                }
            }
        }
        self.image = new_image;
    }

    /// Increment the image index and calls `change_image()`.
    fn next_image(&mut self) {
        self.image_index_current = (self.image_index_current + 1) % self.image_list.len();
        self.change_image();
    }
    /// Decrement the image index and calls `change_image()`.
    fn previous_image(&mut self) {
        if self.image_index_current == 0 {
            self.image_index_current = self.image_list.len() - 1;
        } else {
            self.image_index_current -= 1;
        }
        self.change_image();
    }
    /// Change the image to the one at the current index.
    fn change_image(&mut self) {
        self.image_original = image::open(&self.image_list[self.image_index_current]).unwrap();
        let (img_size, _h) = self.image_solved.dimensions();
        self.image_solved = self.image_original.resize_to_fill(
            img_size,
            img_size,
            image::imageops::FilterType::Nearest,
        );
    }
}

fn main() {
    // for debugging, do `set PUZZLE_LOG=debug` in cmd
    Builder::from_env("PUZZLE_LOG").init();
    debug!("Logger initialized");

    nannou::app(model)
        .update(update)
        .loop_mode(LoopMode::Wait)
        .run();
}

fn model(app: &App) -> Model {
    let args: Vec<_> = env::args().collect();

    // Check if the user passed a size argument
    // If not, use the default size of 4.
    // Grid is always square.
    let grid_size = match args.len() {
        2 => {
            let size = args[1].parse().unwrap();
            size
        }
        _ => 4,
    };

    let _window = app
        .new_window()
        .size(START_WINDOW_SIZE, START_WINDOW_SIZE)
        .title("Sliding Puzzle")
        .view(view)
        .event(event)
        .resized(window_resized)
        .build()
        .unwrap();

    let pad = (app.window_rect().h() * PAD_HEIGHT_FACTOR) as u32;
    let img_size = START_WINDOW_SIZE - 2 * pad;

    // Load a list of images from the images folder.
    // Use the first image as current.
    // If no images are found, use a blank image.
    let mut image_original: image::DynamicImage;
    let image_index_current = 0;
    let image_list = get_images();

    if image_list.is_empty() {
        println!("No images found in the images folder");
        image_original = image::DynamicImage::new_rgba8(img_size, img_size);
        // Fill the image with white
        for x in 0..img_size {
            for y in 0..img_size {
                image_original.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
    } else {
        debug!("Images found: {:?}", image_list);
        image_original = image::open(&image_list[image_index_current]).unwrap();
    }

    // Resize the original image to a square to fit the window,
    // also make a working copy of it which will be used to display the pieces
    let image_solved =
        image_original.resize_to_fill(img_size, img_size, image::imageops::FilterType::Nearest);
    let image = image_solved.clone();
    let texture = wgpu::Texture::from_image(app, &image);

    Model {
        grid_size,
        flag_scramble: false,
        flag_show_numbers: true,
        scramble_count: 0,
        board: solved_board(grid_size),
        image_list,
        image_index_current,
        image_original,
        image_solved,
        image,
        texture,
    }
}

/// Resize the image when the window is resized.
fn window_resized(_app: &App, model: &mut Model, dim: Vec2) {
    let pad = (dim.y * PAD_HEIGHT_FACTOR) as u32;
    let img_size = dim.y.min(dim.x) as u32 - 2 * pad;
    model.image_solved = model.image_original.resize_to_fill(
        img_size,
        img_size,
        image::imageops::FilterType::Nearest,
    );
}

/// Game loop
/// This function is called every frame.
/// It updates the image and the texture.
/// It also scrambles the board if the flag is set.
fn update(app: &App, model: &mut Model, _update: Update) {
    // Do a number of random moves to scramble the board is the flag is set.
    if model.flag_scramble {
        model.do_one_random_move();
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

/// Process a user mouse click.
fn mouse_clicked(mouse_x: f32, mouse_y: f32, app: &App, model: &mut Model) {
    // and move it if it can be moved.
    let win = app.window_rect();
    let pad = win.h() * PAD_HEIGHT_FACTOR;
    let cell_size = (win.h().min(win.w()) - 2.0 * pad) / model.grid_size as f32;
    let board_size = cell_size * model.grid_size as f32;
    if mouse_x.abs().max(mouse_y.abs()) > board_size / 2.0 {
        debug!("Clicked outside the board");
        return;
    }
    let x_offset = (win.w() - 2.0 * pad - board_size) / 2.0;
    let y_offset = (win.h() - 2.0 * pad - board_size) / 2.0;

    let ix_clicked = (model.grid_size as f32 * (mouse_x + win.w() / 2.0 - pad - x_offset)
        / (win.w() - 2.0 * pad - 2.0 * x_offset)) as usize;
    let iy_clicked = (model.grid_size as f32 * (mouse_y + win.h() / 2.0 - pad - 2.0 * y_offset)
        / (win.h() - 2.0 * pad - y_offset)) as usize;
    debug!("Indices clicked: {}, {}", ix_clicked, iy_clicked);
    model.try_move(ix_clicked, iy_clicked);
}

fn event(app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        MousePressed(_button) => mouse_clicked(app.mouse.x, app.mouse.y, app, model),
        KeyPressed(Key::R) => model.reset(),
        KeyPressed(Key::N) => model.flag_show_numbers = !model.flag_show_numbers,
        KeyPressed(Key::Period) => model.next_image(),
        KeyPressed(Key::Comma) => model.previous_image(),
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
            if model.flag_show_numbers {
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
    }

    draw.to_frame(app, &frame).unwrap();
}

/// Get the list of images from the images folder.
/// Only PNG images are accepted.
/// If no images are found, an empty vector is returned.
fn get_images() -> Vec<PathBuf> {
    let mut images = vec![];
    match fs::read_dir("images") {
        Ok(paths) => {
            for path in paths {
                let path = path.unwrap().path();
                if path.extension().unwrap() == "png" {
                    images.push(path);
                }
            }
        }
        Err(e) => {
            println!("Error reading images folder: {e}");
        }
    }
    images
}
