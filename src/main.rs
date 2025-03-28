use std::{env, time, thread};
use nannou::prelude::*;


static PAD_HEIGHT_FACTOR: f32 = 0.1;


// Build a solved board with numbers up to height * width - 1
fn solved_board(height: usize, width: usize) -> Vec<Vec<usize>> {
    let mut board = vec![vec![0; width]; height];
    for row in 0..height {
        for col in 0..width {
            board[row][col] = (height - row - 1) * width + col + 1;
        }
    }
    board[0][width - 1] = 0;
    board
}

// Build a board with numbers up to height * width - 1
// in a scrambled order.
// TODO: Not sure the resultant board is solvable!
//fn scrambled_board(height: usize, width: usize) -> Vec<Vec<usize>> {
//    // vector of numbers from 0 to height * width - 1 in a random order
//    let mut numbers: Vec<usize> = (0..height * width).collect();
//    numbers.shuffle(&mut thread_rng());
//
//    let mut board = vec![vec![0; width]; height];
//    for i in 0..height {
//        for j in 0..width {
//            board[i][j] = numbers.pop().unwrap();
//        }
//    }
//    board
//}

struct Model {
    height: usize,
    width: usize,
    flag_scramble: bool,
    scramble_count: usize,
    board: Vec<Vec<usize>>,
}
impl Model {
    fn new(h: usize, w: usize) -> Self {
        Self {
            height: h,
            width: w,
	    flag_scramble: false,
	    scramble_count: 0,
            board: solved_board(h, w),
        }
    }
    /// Reset board
    fn reset(&mut self) {
        self.board = solved_board(self.height, self.width);
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
	    let ix = random_range(0, self.width);
	    let iy = random_range(0, self.height);
	    if self.is_move_valid(ix, iy) {
		self.try_move(ix, iy);
		return
	    }
	}
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

    let (height, width) = match args.len() {
        2 => {
            let size = args[1].parse().unwrap();
            (size, size)
        }
        3 => {
            let height = args[1].parse().unwrap();
            let width = args[2].parse().unwrap();
            (height, width)
        }
        _ => (4, 4),
    };

    let _window = app
        .new_window()
        .size(300, 300)
        .title("Sliding Puzzle")
        .view(view)
        .event(event)
        .build()
        .unwrap();

    Model::new(height, width)
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
}

fn event(app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        MousePressed(_button) => {
            // Check if the user clicked on an arrow
            // and move it if it can be moved.
            let win = app.window_rect();
            let pad = win.h() * PAD_HEIGHT_FACTOR;
            let w = win.w();
            let h = win.h();
            if (win.top() - app.mouse.y)
                .min(app.mouse.y - win.bottom())
                .min(win.right() - app.mouse.x)
                .min(app.mouse.x - win.left())
                < pad
            {
                //println!("Clicked outside the board");
                return;
            }
            let ix_clicked =
                (model.width as f32 * (app.mouse.x + w / 2.0 - pad) / (w - 2.0 * pad)) as usize;
            let iy_clicked =
                (model.height as f32 * (app.mouse.y + h / 2.0 - pad) / (h - 2.0 * pad)) as usize;
            //println!("Indices clicked: {}, {}", ix_clicked, iy_clicked);
            model.try_move(ix_clicked, iy_clicked);
        }
        KeyPressed(Key::R) => model.reset(),
	KeyPressed(Key::S) => {
	    app.set_loop_mode(LoopMode::RefreshSync);
	    model.flag_scramble = true;
	},
        _ => (),
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    // draw the board
    let win = app.window_rect();
    let pad = win.h() * PAD_HEIGHT_FACTOR;
    let cell_width = (win.w() - 2.0 * pad) / model.width as f32;
    let cell_height = (win.h() - 2.0 * pad) / model.height as f32;
    let font_size = (cell_height.min(cell_width) / 2.0) as u32;

    // draw all the cells
    for row in 0..model.height {
        let y = win.bottom() + pad + row as f32 * cell_height + cell_height / 2.0;

        for col in 0..model.width {
            let x = win.left() + pad + col as f32 * cell_width + cell_width / 2.0;

            let piece = model.board[row][col];

            let fill_color = match piece {
                0 => GREY,
                _ => WHITE,
            };

            // draw the cell
            draw.rect()
                .x_y(x, y)
                .w_h(cell_width, cell_height)
                .color(fill_color)
                .stroke(GREY)
                .stroke_weight(2.0);

            // draw the number of the piece

            let text = match piece {
                0 => String::from(""),
                _ => piece.to_string(),
            };

            let text_area = geom::Rect::from_w_h(cell_width, cell_height).relative_to([-x, -y]);

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
