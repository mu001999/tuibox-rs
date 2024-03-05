use std::process::exit;

use tuibox::{Mouse, Position, UIBox, UI};

fn draw(b: &mut UIBox) -> String {
    let mut out = String::new();
    for y in 0..b.size.h {
        for x in 0..b.size.w {
            out += &format!(
                "\x1b[48;2;{};{};{}m ",
                (255.0 * (x as f64 / b.size.w as f64)).round() as i32,
                (255.0 * (y as f64 / b.size.h as f64)).round() as i32,
                (255.0 * ((x * y) as f64 / (b.size.w as i64 * b.size.h as i64) as f64)).round()
                    as i32,
            );
        }
        out += "\x1b[0m\n";
    }
    out
}

fn click(u: &mut UI, b: &mut UIBox, _: i32, _: i32, _: Mouse) {
    b.data1 = String::from("\x1b[0m                \n  you clicked me!  \n                ");
    b.state_next = 1;
    u.draw_one(b, true);
}

fn hover(u: &mut UI, b: &mut UIBox, _: i32, _: i32) {
    b.data1 = String::from("\x1b[0m                \n  you hovered me!  \n                ");
    b.state_next = 2;
    u.draw_one(b, true);
}

fn stop(_: &mut UI) {
    exit(0);
}

fn main() {
    let mut ui = UI::new(0);

    ui.add(
        Position::Specific(1),
        Position::Specific(1),
        ui.size,
        0,
        Some(draw),
        None,
        None,
        String::new(),
        String::new(),
    );

    ui.text(
        Position::Specific(ui.center_x(19)),
        Position::Center,
        String::from("\x1b[0m                   \n    click on me!   \n                   "),
        0,
        Some(click),
        Some(hover),
    );

    ui.key('q', stop);

    ui.draw();

    ui.run();
}
