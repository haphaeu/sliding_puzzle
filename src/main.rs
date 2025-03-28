use nannou::prelude::*;

static PAD_HEIGHT_FACTOR: f32 = 0.1;
static GRID_HEIGHT: usize = 4;
static GRID_WIDTH: usize = 4;

fn board() -> Vec<Vec<usize>> {
    let mut board = vec![vec![0; GRID_WIDTH]; GRID_HEIGHT];
    let mut count = 1;
    for row in 0..GRID_HEIGHT {
        for col in 0..GRID_WIDTH {
            board[row][col] = count;
            count += 1;
        }
    }
    board[GRID_HEIGHT - 1][GRID_WIDTH - 1] = 0;
    board
}

struct Model {
    board: Vec<Vec<usize>>,
}
impl Model {
    fn new() -> Self {
        Self {
            board: board(),
        }
    }
    /// Reset board
    fn reset(&mut self) {
        self.board = board();
    }
    
    /// Returns the indices of the empty space.
    fn index_empty(&self) -> (usize, usize) {

        let iy = self.board.iter().position(|r| r.contains(&0)).unwrap();
        let ix = self.board[iy].iter().position(|&x| x == 0).unwrap();
       
        (ix, iy)
    }

    /// When the user clicks on a piece, this function checks
    /// if that piece can be moved and returns `true` if the arrow
    // can be moved, and `false` otherwise.
    fn is_move_valid(&self, ix: usize, iy: usize) -> bool {
        let (empty_x, empty_y) = self.index_empty();

        ix.abs_diff(empty_x) + iy.abs_diff(empty_y) == 1
    }

    /// Move the arrow at `index` to the empty space.
    /// Check if the move is valid.
    fn try_move(&mut self, ix: usize, iy: usize) {
        println!("Trying to move piece at index {ix}, {iy}");
        match self.is_move_valid(ix, iy) {
            true => {
                println!("Move is valid");
                let (empty_x, empty_y) = self.index_empty();
                self.board[empty_y][empty_x] = self.board[iy][ix];
                self.board[iy][ix] = 0;
            }
            false => {
                println!("Move is invalid");
                ()
            }
        }
    }
}

fn main() {
    nannou::app(model).loop_mode(LoopMode::Wait).run();
}

fn model(app: &App) -> Model {
    let _window = app
        .new_window()
        .size(300, 300)
        .title("Sliding Puzzle")
        .view(view)
        .event(event)
        .build()
        .unwrap();
    Model::new()
}

fn event(app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        MousePressed(_button) => {
            // Check if the user clicked on an arrow
            // and move it if it can be moved.
            println!("Playing mode");
            println!("Clicked at {}, {}", app.mouse.x, app.mouse.y);
            let win = app.window_rect();
            let pad = win.h() * PAD_HEIGHT_FACTOR;
            let w = win.w();
            let h = win.h();
            if (win.top() - app.mouse.y)
                .min(app.mouse.y - win.bottom())
                .min(win.right() - app.mouse.x)
                .min(app.mouse.x - win.left()) < pad 
            {
                println!("Clicked outside the board");
                return;
            }
            let ix_clicked = (GRID_WIDTH as f32 * (app.mouse.x + w / 2.0) / w) as usize;
            let iy_clicked = (GRID_HEIGHT as f32 * (app.mouse.y + h / 2.0) / h) as usize;
            println!("Indices clicked: {}, {}", ix_clicked, iy_clicked);
            model.try_move(ix_clicked, iy_clicked);
        },
        KeyPressed(Key::R) => model.reset(),
        _ => (),
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    // draw the board
    let win = app.window_rect();
    let pad = win.h() * PAD_HEIGHT_FACTOR;
    let cell_width = (win.w() - 2.0 * pad) / GRID_WIDTH as f32;
    let cell_height = (win.h() - 2.0 * pad) / GRID_HEIGHT as f32;
    let font_size = (cell_height.min(cell_width) / 2.0) as u32;

    // draw all the cells
    for row in 0..GRID_HEIGHT {
        let y = win.bottom() + pad + row as f32 * cell_height + cell_height / 2.0;
        for col in 0..GRID_WIDTH {
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

            let text_area = geom::Rect::from_w_h(cell_width, cell_height)
                .relative_to([-x, -y]);

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
