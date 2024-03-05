use std::{
    io::{Read, Write},
    // mem::transmute,
    // os::raw::c_ushort,
    str::Split,
};

use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW};

type OnClick = fn(&mut UI, &mut UIBox, i32, i32, Mouse);
type OnHover = fn(&mut UI, &mut UIBox, i32, i32);
type OnEvent = fn(&mut UI);

#[derive(Debug, Clone, Copy)]
pub enum Mouse {
    Up,
    Down,
}

#[derive(Clone, Debug, Copy)]
pub struct Size {
    pub w: i32,
    pub h: i32,
}

pub enum Position {
    Center,
    Specific(i32),
}

#[derive(Clone, Debug)]
pub struct UIBox {
    _id: i32,
    x: i32,
    y: i32,
    pub size: Size,
    screen: i32,
    cache: String,
    pub state_cur: i32,
    pub state_next: i32,
    draw: Option<fn(&mut UIBox) -> String>,
    onclick: Option<OnClick>,
    onhover: Option<OnHover>,
    pub data1: String,
    pub data2: String,
}

impl UIBox {
    fn contains(&self, x: i32, y: i32) -> bool {
        self.x <= x && x <= self.x + self.size.w && self.y <= y && y <= self.y + self.size.h
    }
}

#[derive(Debug)]
struct UIEvt {
    chr: char,
    f: OnEvent,
}

#[derive(Debug)]
pub struct UI {
    tio: Termios,
    pub size: Size,
    boxs: Vec<UIBox>,
    evts: Vec<UIEvt>,
    click: Option<UIBox>,
    screen: i32,
    scroll: i32,
    canscroll: bool,
    id: i32,
    force: bool,
}

impl UI {
    /// Initializes a new UI struct
    pub fn new(s: i32) -> UI {
        let size = get_winsize();
        let tio = Termios::from_fd(libc::STDIN_FILENO).expect("termios from fd");

        let mut raw = tio;
        raw.c_lflag &= !(ECHO | ICANON);
        tcsetattr(libc::STDIN_FILENO, TCSANOW, &mut raw).expect("tcsetttr failed");

        print!("\x1b[?1049h\x1b[0m\x1b[2J\x1b[?1003h\x1b[?1015h\x1b[?1006h\x1b[?25l");

        UI {
            tio,
            size,
            boxs: Vec::new(),
            evts: Vec::new(),
            click: None,
            screen: s,
            scroll: 0,
            canscroll: true,
            id: 0,
            force: false,
        }
    }

    /// Adds a new box to the UI.
    pub fn add(
        &mut self,
        x: Position,
        y: Position,
        size: Size,
        state: i32,
        draw: Option<fn(&mut UIBox) -> String>,
        onclick: Option<OnClick>,
        onhover: Option<OnHover>,
        data1: String,
        data2: String,
    ) -> i32 {
        let id = self.id;

        let mut b = UIBox {
            _id: id,
            x: match x {
                Position::Center => self.center_x(size.w),
                Position::Specific(x) => x,
            },
            y: match y {
                Position::Center => self.center_y(size.h),
                Position::Specific(y) => y,
            },
            size,
            screen: self.screen,
            state_cur: state,
            state_next: state,
            draw,
            onclick,
            onhover,
            data1,
            data2,
            cache: String::new(),
        };

        if let Some(f) = draw {
            b.cache = f(&mut b);
        }

        self.boxs.push(b);
        self.id += 1;

        id
    }

    pub fn center_x(&self, w: i32) -> i32 {
        (self.size.w as i32 - w) / 2
    }

    pub fn center_y(&self, h: i32) -> i32 {
        (self.size.h as i32 - h) / 2
    }

    pub fn text(
        &mut self,
        x: Position,
        y: Position,
        str: String,
        state: i32,
        click: Option<OnClick>,
        hover: Option<OnHover>,
    ) -> i32 {
        self.add(
            x,
            y,
            Size {
                w: str.len() as i32,
                h: 1,
            },
            state,
            Some(Self::_text),
            click,
            hover,
            str,
            String::new(),
        )
    }

    /// Draws all boxes to the screen.
    pub fn draw(&mut self) {
        print!("\x1b[0m\x1b[2J");
        let mut boxs = std::mem::take(&mut self.boxs);
        for b in &mut boxs {
            self.draw_one(b, false);
        }
        self.boxs.append(&mut boxs);
        std::io::stdout().flush().expect("flush stdout");
        self.force = false;
    }

    /// Draws a single box to the screen.
    pub fn draw_one(&mut self, b: &mut UIBox, flush: bool) {
        if b.screen != self.screen {
            return;
        }

        let mut buf = String::new();
        // cache outdated
        if self.force || b.state_next != b.state_cur {
            if let Some(draw) = b.draw {
                buf = draw(b);
            }
            b.cache = buf.clone();

            b.state_cur = b.state_next;
        } else {
            buf = b.cache.clone();
        }

        let mut n = 0;
        for tok in buf.split('\n') {
            let cursor_y = self.cursor_y(b, n);
            if 1 <= b.x
                && b.x <= self.size.w as i32
                && 1 <= cursor_y
                && cursor_y <= self.size.h as i32
            {
                print!("\x1b[{};{}H{}", cursor_y, b.x, tok);
                n += 1;
            }
        }

        if flush {
            std::io::stdout().flush().expect("flush stdout");
        }
    }

    /// Forces a redraw of the screen, updating all boxes' caches.
    pub fn redraw(&mut self) {
        self.force = true;
        self.draw();
    }

    /// Adds a new key event listener to the UI.
    pub fn key(&mut self, chr: char, f: OnEvent) {
        self.evts.push(UIEvt { chr, f });
    }

    /// Clears all elements from the UI.
    pub fn clear(&mut self) {
        *self = Self::new(self.screen);
    }

    pub fn run(&mut self) {
        loop {
            let mut buffer = vec![0u8; 64];
            std::io::stdin().read(&mut buffer).unwrap();
            self.update(
                String::from_utf8(buffer)
                    .unwrap()
                    .trim_end_matches(0 as char)
                    .to_owned(),
            );
        }
    }
}

impl UI {
    fn _text(b: &mut UIBox) -> String {
        std::mem::take(&mut b.data1)
    }

    fn cursor_y(&self, b: &UIBox, n: i32) -> i32 {
        if self.canscroll {
            b.y + n + self.scroll
        } else {
            b.y + n
        }
    }

    fn cursor(&self, toks: &mut Split<char>) -> (i32, i32) {
        let x = toks.next().unwrap().parse().unwrap();

        let tok = toks.next().unwrap();
        let m_pos = tok.find(|c: char| !c.is_ascii_digit()).unwrap();
        let mut y: i32 = tok[..m_pos].parse().unwrap();

        if self.canscroll {
            y -= self.scroll;
        }

        (x, y)
    }

    fn mouse_up(&mut self, (x, y): (i32, i32)) {
        let Some(mut b) = self.click.take() else {
            return;
        };

        if !b.contains(x, y) {
            return;
        }

        if let Some(f) = b.onclick {
            f(self, &mut b, x, y, Mouse::Up);
        }
    }

    fn mouse_down_first(&mut self, (x, y): (i32, i32)) {
        let mut boxs = std::mem::take(&mut self.boxs);
        for b in &mut boxs {
            if b.screen != self.screen {
                continue;
            }

            if !b.contains(x, y) {
                continue;
            }

            if let Some(f) = b.onclick {
                f(self, b, x, y, Mouse::Down);
                self.click = Some(b.clone());
                break;
            };
        }
        self.boxs.append(&mut boxs);
    }

    fn mouse_down_moving(&mut self, (x, y): (i32, i32)) {
        if let Some(mut b) = self.click.take() {
            if let Some(f) = b.onclick {
                f(self, &mut b, x, y, Mouse::Down);
            }
            self.click = Some(b);
        }
    }

    fn mouse_hover(&mut self, (x, y): (i32, i32)) {
        let mut boxs = std::mem::take(&mut self.boxs);
        for b in &mut boxs {
            if b.screen != self.screen {
                continue;
            }

            if !b.contains(x, y) {
                continue;
            }

            if let Some(f) = b.onhover {
                f(self, b, x, y);
            };
        }
        self.boxs.append(&mut boxs);
    }

    fn update(&mut self, c: String) {
        if !c.starts_with("\x1b[<") {
            for f in self
                .evts
                .iter()
                .filter_map(|evt| {
                    if c.starts_with(evt.chr) {
                        Some(evt.f)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
            {
                f(self);
            }
            return;
        }

        let mut toks = c[3..].split(';');
        let Some(tok) = toks.next() else {
            return;
        };

        if self.canscroll && tok.as_bytes()[0] == b'6' {
            if tok.as_bytes()[1] == b'4' {
                self.scroll += 2;
            } else {
                self.scroll -= 2;
            }

            print!("\x1b[0m\x1b[2J");
            self.draw();
            return;
        }

        if tok.as_bytes()[0] == b'0' && c.find('m').is_some() {
            self.mouse_up(self.cursor(&mut toks));
        } else if tok.as_bytes()[0] == b'0' {
            self.mouse_down_first(self.cursor(&mut toks));
        } else if tok.starts_with("32") {
            self.mouse_down_moving(self.cursor(&mut toks));
        } else if tok.starts_with("35") {
            self.mouse_hover(self.cursor(&mut toks));
        }
    }
}

impl Drop for UI {
    /// Frees the given UI struct, and takes the terminal out of raw mode.
    fn drop(&mut self) {
        print!("\x1b[0m\x1b[2J\x1b[?1049l\x1b[?1003l\x1b[?1015l\x1b[?1006l\x1b[?25h");
        tcsetattr(libc::STDIN_FILENO, TCSANOW, &mut self.tio).expect("tcsetattr");

        for (key, value) in std::env::vars() {
            if key == "TERM" && (value == "screen" || value == "tmux") {
                println!(
                    "Note: Terminal multiplexer detected.
  For best performance (i.e. reduced flickering), running natively inside
  a GPU-accelerated terminal such as alacritty or kitty is recommended."
                );
            }
        }
    }
}

fn get_winsize() -> Size {
    // FIXME: tiocgwinsz not works on my pc
    // let (ws_row, ws_col): (c_ushort, c_ushort) =
    //     unsafe { transmute(ioctls::tiocgwinsz(libc::STDIN_FILENO)) };
    Size {
        // w: ws_col as i32,
        // h: ws_row as i32,
        w: 80,
        h: 24,
    }
}
