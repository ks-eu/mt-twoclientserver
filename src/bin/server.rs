use std::thread;
use std::net::{Shutdown, SocketAddr, TcpListener};
use std::io::{BufReader, prelude::*};
use std::sync::{Arc, mpsc, Mutex};
use std::collections::HashMap;

enum TcpInput {
    Connected(mpsc::Sender<String>, SocketAddr),
    Message(String, SocketAddr),
    Error(String, SocketAddr),
    Disconnected(SocketAddr),
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878")
                               .expect("Failed to bind to port");
    let connections = Arc::new(Mutex::new(0));
    let (send, recv) = mpsc::channel::<TcpInput>();
    let route_con = Arc::clone(&connections);
    thread::spawn(move || {
        let mut clients: HashMap<SocketAddr, mpsc::Sender<String>> = HashMap::new();
        loop {
            match recv.recv() {
                Ok(TcpInput::Connected(sender, addr)) => {
                    println!("new client, from address {addr}");
                    clients.insert(addr, sender);
                }
                Ok(TcpInput::Message(msg, addr)) => {
                    println!("from {addr}: {msg}");
                    for (h_addr, h_sender) in &clients {
                        if *h_addr != addr {
                            let _ = h_sender.send(msg.clone());
                        }
                    }
                }
                Ok(TcpInput::Error(e, addr)) => {
                    println!("from {addr}: error = {e}");
                }
                Ok(TcpInput::Disconnected(addr)) => { 
                    if clients.remove(&addr).is_some() {
                        println!("client disconnected, address {addr}");
                        let mut count = route_con.lock()
                                                 .expect("poisoned mutex");
                        *count -= 1;
                        drop(count);
                    }
                }
                Err(e) => {
                    println!("error: {e}");
                    break;
                }
            }
        }
    });



    loop {
        let (stream, address) = listener.accept()
                                        .expect("failed to accept stream");
        let mut count = connections.lock()
                                   .expect("poisoned mutex");

        if *count >= 2 {
            println!("max connections reached, rejecting {address}");
            drop(count);
            drop(stream);
            continue;
        }
        
        let (w_send, w_recv) = mpsc::channel::<String>();
        let _ = send.send(TcpInput::Connected(w_send, address));

        *count += 1;
        drop(count);
        
        let c_send = send.clone();
        let cr_stream = stream.try_clone().unwrap(); 
        thread::spawn(move || {
            let buf_reader = BufReader::new(&cr_stream);
            for line in buf_reader.lines() {
                match line {
                    Ok(msg) => {
                        let _ = c_send.send(TcpInput::Message(msg, address));
                    }
                    Err(e) => {
                        let _ = c_send.send(TcpInput::Error(e.to_string(), address));
                        let _ = cr_stream.shutdown(Shutdown::Both);
                        break;
                    }
                }
            }
            let _ = c_send.send(TcpInput::Disconnected(address));
        });

        let cw_send = send.clone();
        let mut cw_stream = stream.try_clone().unwrap();
        thread::spawn(move || {
            loop {
                match w_recv.recv() {
                    Ok(input) => {
                        if let Err(e) = cw_stream.write_all(input.trim().as_bytes()) {
                            let _ = cw_send.send(TcpInput::Error(e.to_string(), address));
                            let _ = cw_stream.shutdown(Shutdown::Both);
                            break;
                        }
                        if let Err(e) = cw_stream.write_all(b"\n") {
                            let _ = cw_send.send(TcpInput::Error(e.to_string(), address));
                            let _ = cw_stream.shutdown(Shutdown::Both);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("error {e}");
                        break;
                    }
                }
            }
            let _ = cw_send.send(TcpInput::Disconnected(address));
        });
    }
}
