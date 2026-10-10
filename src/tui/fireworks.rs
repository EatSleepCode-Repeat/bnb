use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Color, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, stdout, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct FastRng(u64);

impl FastRng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(123_456_789);
        Self(if seed == 0 {
            0x1234_5678_9abc_def0
        } else {
            seed
        })
    }

    fn gen_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn gen_range_usize(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        min + (self.gen_u64() as usize % (max - min))
    }

    fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        if min >= max {
            return min;
        }
        let norm = (self.gen_u64() % 10_000) as f32 / 10_000.0;
        min + norm * (max - min)
    }

    fn gen_bool(&mut self, p: f64) -> bool {
        (self.gen_u64() % 100) < (p * 100.0) as u64
    }
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    color: Color,
    symbol: char,
    life: usize,
}

struct Rocket {
    x: f32,
    y: f32,
    target_y: f32,
    vy: f32,
    color: Color,
}

const CYBER_COLORS: &[Color] = &[
    Color::Rgb {
        r: 255,
        g: 0,
        b: 128,
    }, // Neon Pink
    Color::Rgb {
        r: 0,
        g: 255,
        b: 240,
    }, // Neon Cyan
    Color::Rgb {
        r: 255,
        g: 230,
        b: 0,
    }, // Neon Yellow
    Color::Rgb {
        r: 157,
        g: 0,
        b: 255,
    }, // Cyber Purple
    Color::Rgb {
        r: 0,
        g: 255,
        b: 100,
    }, // Matrix Green
    Color::Rgb {
        r: 255,
        g: 80,
        b: 0,
    }, // Electric Orange
];

const PARTICLE_SYMBOLS: &[char] = &['✦', '★', '✧', '•', '*', 'x', '.'];

pub fn run_fireworks() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    stdout.execute(cursor::Hide).map_err(|e| e.to_string())?;

    let res = animate_fireworks(&mut stdout);

    let _ = stdout.execute(cursor::Show);
    let _ = stdout.execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();

    res
}

fn animate_fireworks(stdout: &mut io::Stdout) -> Result<(), String> {
    let mut rng = FastRng::new();
    let mut rockets: Vec<Rocket> = Vec::new();
    let mut particles: Vec<Particle> = Vec::new();

    loop {
        let (cols, rows) = size().unwrap_or((80, 24));
        if cols < 20 || rows < 10 {
            break;
        }

        if event::poll(Duration::from_millis(33)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c')
                ) {
                    break;
                }
            }
        }

        write!(stdout, "\x1b[2J").map_err(|e| e.to_string())?;

        if rockets.len() < 5 && rng.gen_bool(0.35) {
            let rx = rng.gen_range_f32(6.0, cols.saturating_sub(6) as f32);
            let target = rng.gen_range_f32(3.0, rows.saturating_sub(8) as f32);
            let color = CYBER_COLORS[rng.gen_range_usize(0, CYBER_COLORS.len())];
            rockets.push(Rocket {
                x: rx,
                y: (rows - 2) as f32,
                target_y: target,
                vy: -1.2,
                color,
            });
        }

        let mut exploded_indices = Vec::new();
        for (idx, rocket) in rockets.iter_mut().enumerate() {
            rocket.y += rocket.vy;
            if rocket.y <= rocket.target_y {
                exploded_indices.push(idx);
            }
        }

        for idx in exploded_indices.into_iter().rev() {
            let rocket = rockets.remove(idx);
            let num_particles = rng.gen_range_usize(28, 48);
            for _ in 0..num_particles {
                let angle = rng.gen_range_f32(0.0, std::f32::consts::TAU);
                let speed = rng.gen_range_f32(0.5, 2.0);
                let vx = angle.cos() * speed * 1.5;
                let vy = angle.sin() * speed;
                let symbol = PARTICLE_SYMBOLS[rng.gen_range_usize(0, PARTICLE_SYMBOLS.len())];
                let life = rng.gen_range_usize(16, 32);

                particles.push(Particle {
                    x: rocket.x,
                    y: rocket.y,
                    vx,
                    vy,
                    color: rocket.color,
                    symbol,
                    life,
                });
            }
        }

        for p in particles.iter_mut() {
            p.x += p.vx;
            p.y += p.vy;
            p.vy += 0.06;
            p.vx *= 0.95;
            if p.life > 0 {
                p.life -= 1;
            }
        }
        particles.retain(|p| {
            p.life > 0
                && p.x >= 1.0
                && p.x < (cols - 1) as f32
                && p.y >= 1.0
                && p.y < (rows - 1) as f32
        });

        let title = " ⚡ CYBERPUNK 2077 // BNB-SHELL EASTER EGG ⚡ ";
        let banner_x = (cols as usize).saturating_sub(title.len()) / 2;
        stdout
            .queue(cursor::MoveTo(banner_x as u16, 1))
            .map_err(|e| e.to_string())?;
        stdout
            .queue(SetForegroundColor(Color::Rgb {
                r: 0,
                g: 255,
                b: 240,
            }))
            .map_err(|e| e.to_string())?;
        write!(stdout, "{}", title).map_err(|e| e.to_string())?;

        let footer = "[ Press ESC or 'q' to return to shell ]";
        let footer_x = (cols as usize).saturating_sub(footer.len()) / 2;
        stdout
            .queue(cursor::MoveTo(footer_x as u16, rows.saturating_sub(1)))
            .map_err(|e| e.to_string())?;
        stdout
            .queue(SetForegroundColor(Color::Rgb {
                r: 120,
                g: 120,
                b: 150,
            }))
            .map_err(|e| e.to_string())?;
        write!(stdout, "{}", footer).map_err(|e| e.to_string())?;

        for r in &rockets {
            if r.x >= 1.0 && r.x < (cols - 1) as f32 && r.y >= 1.0 && r.y < (rows - 1) as f32 {
                stdout
                    .queue(cursor::MoveTo(r.x as u16, r.y as u16))
                    .map_err(|e| e.to_string())?;
                stdout
                    .queue(SetForegroundColor(r.color))
                    .map_err(|e| e.to_string())?;
                write!(stdout, "▲").map_err(|e| e.to_string())?;
            }
        }

        for p in &particles {
            stdout
                .queue(cursor::MoveTo(p.x as u16, p.y as u16))
                .map_err(|e| e.to_string())?;
            stdout
                .queue(SetForegroundColor(p.color))
                .map_err(|e| e.to_string())?;
            write!(stdout, "{}", p.symbol).map_err(|e| e.to_string())?;
        }

        stdout.flush().map_err(|e| e.to_string())?;
    }

    Ok(())
}
