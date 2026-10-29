use std::thread;
use std::sync::mpsc;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    queue,
};

const HISTORY_CAPACITY: usize = 200;
fn add_message(history: &mut VecDeque<String>, message: String) {
    if history.len() == HISTORY_CAPACITY {
        history.pop_front();
    }

    history.push_back(message);
}

enum InputEvent {
    Message(String),
    KeyStroke(char),
    BackSpace,
    Send,
    Exit,
    Error(String),
}

fn main() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode().unwrap();
    print!("\x1B[2J\x1B[H\x1B[?25h");
    io::stdout().flush().expect("failed to clear screen");

    let (sender, receiver) = mpsc::channel::<InputEvent>();
    let mut stream = TcpStream::connect("127.0.0.1:7878")?;
    
    let read_stream = stream.try_clone()?;
    let tcp_send = sender.clone();
    thread::spawn(move || {
        let buf_reader = BufReader::new(read_stream);
        for line in buf_reader.lines() {
            match line {
                Ok(msg) => {
                    if msg.is_empty() {
                        continue;
                    }
                    let _ = tcp_send.send(InputEvent::Message(msg));
                }
                Err(e) => {
                    let _ = tcp_send.send(InputEvent::Error(e.to_string()));
                    break;
                }
            }
        }
    });

    let input_send = sender.clone();
    thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(KeyEvent { code: KeyCode::Char(c), .. })) => {
                    let _ = input_send.send(InputEvent::KeyStroke(c));
                }
                Ok(Event::Key(KeyEvent { code: KeyCode::Backspace, .. })) => {
                    let _ = input_send.send(InputEvent::BackSpace);
                }
                Ok(Event::Key(KeyEvent { code: KeyCode::Enter, .. })) => {
                    let _ = input_send.send(InputEvent::Send);
                }
                Ok(Event::Key(KeyEvent { code: KeyCode::Esc, .. })) => {
                    let _ = input_send.send(InputEvent::Exit);
                }
                Ok(_) => {}
                Err(error) => {
                    let _ = input_send.send(InputEvent::Error(error.to_string()));
                    break;
                }
            }
        }
    });

    let mut history: VecDeque<String> = VecDeque::with_capacity(HISTORY_CAPACITY);
    let mut input = String::new();

    let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
    let input_row = height.saturating_sub(1);
    let mut stdout = io::stdout();
    
    queue!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, input_row),
        crossterm::style::Print("> "),
        crossterm::cursor::MoveTo(2, input_row)
    )?;

    stdout.flush()?;

    loop {
        match receiver.recv() {
            Ok(InputEvent::KeyStroke(c)) => {
                input.push(c);
            }
            Ok(InputEvent::BackSpace) => {
                input.pop();
            }
            Ok(InputEvent::Send) => {
                add_message(&mut history, format!("You> {}", input.trim()));
                stream.write_all(input.trim().as_bytes())?;
                stream.write_all(b"\n")?;
                input.clear();
            }
            Ok(InputEvent::Message(msg)) => {
                add_message(&mut history, format!("Friend> {}", msg.trim()));
            }
            Ok(InputEvent::Exit) => {
                break;
            }
            Ok(InputEvent::Error(e)) => {
                eprintln!("failed to read terminal event: {e}");
                break;
            }
            Err(e) => {
                eprintln!("failed to read from reciever: {e}");
                break;
            }
        }
    	let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
    	let input_row = height.saturating_sub(1);
    	let history_rows = input_row as usize;
    
    	let mut stdout = io::stdout();
    
    
    	for row in 0..history_rows {
    		queue!(
    			stdout,
    			crossterm::cursor::MoveTo(0, row as u16),
    			crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
    		)?;
    	}
    
    	for (row, message) in history
    		.iter()
    		.rev()
    		.take(history_rows)
    		.enumerate()
    	{
    		let terminal_row = input_row - 1 - row as u16;
    
    		queue!(
    			stdout,
    			crossterm::cursor::MoveTo(0, terminal_row),
    			crossterm::style::Print(message)
    		)?;
    	}
    
    	queue!(
    		stdout,
    		crossterm::cursor::MoveTo(0, input_row),
    		crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
    		crossterm::style::Print("> "),
    		crossterm::style::Print(&input),
            crossterm::cursor::MoveTo((input.len() + 2) as u16, input_row)
    	)?;

	    stdout.flush()?;
    }
    queue!(
		io::stdout(),
		crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
		crossterm::cursor::MoveTo(0, 0),
		crossterm::cursor::Show
	)?;

	io::stdout().flush()?;
	crossterm::terminal::disable_raw_mode()?;

	Ok(())
}
