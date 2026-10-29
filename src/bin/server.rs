use std::thread;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, prelude::*};

fn handle_connection(stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    for line in buf_reader.lines() {
        match line {
            Ok(msg) => {
                if msg.is_empty() {
                    continue;
                }
                println!("new message: {}", msg);
            }
            Err(e) => {
                println!("connection error or client disconnected: {e}");
                break;
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind to port");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        thread::spawn(move || {
            handle_connection(stream);
        });
    }
}
