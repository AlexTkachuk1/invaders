use std::{error::Error, io, time::{Duration, Instant}, sync::mpsc};
use crossterm::{terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, cursor::{Hide, Show}, event::{self, Event, KeyCode}};
use invaders::{render, frame::{self, new_frame, Drawable}, invaders::Invaders};
use invaders::player::Player;
use rusty_audio::Audio;
use std::thread::{spawn, sleep};

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();

    audio.add("explode", "./sounds/explode.wav");
    audio.add("lose", "./sounds/lose.wav");
    audio.add("move", "./sounds/move.wav");
    audio.add("pew", "./sounds/pew.wav");
    audio.add("startup", "./sounds/startup.wav");
    audio.add("win", "./sounds/win.wav");

    audio.play("startup");

    // Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    // Render loop in a separate thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        loop {
            let current_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &current_frame, false);
            last_frame = current_frame;
        }
    });

    // Game Loop
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    'gameLoop:  loop {
        // Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut current_frame = new_frame();

        // Input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left | KeyCode::Char('a') => player.move_left(),
                    KeyCode::Right | KeyCode::Char('d') => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    },
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameLoop;
                    },
                    _ => {}
                }
            }
        }

        // Updates
        player.update(delta);
        
        if invaders.update(delta) {
            audio.play("move");
        }

        if player.detect_hits(&mut invaders) {
            audio.play("explode");
        }

        // Draw & render 
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];

        for drawable in drawables {
            drawable.draw(&mut current_frame);
        }

        let _ = render_tx.send(current_frame);
        sleep(Duration::from_millis(1));

        // Win or lose?
        if invaders.all_killed() {
            audio.play("win");
            break 'gameLoop;
        }

        if invaders.reached_bottom() {
            audio.play("lose");
            break 'gameLoop;
        }
    }

    // Cleanup 
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
