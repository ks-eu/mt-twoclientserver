use std::io::{self,Write};
use std::net::TcpStream;

fn main () -> io::Result<()> {
    print!("\x1B[2J\x1B[H\x1B[?25h");
    io::stdout().flush().unwrap();
    let mut stream = TcpStream::connect("127.0.0.1:7878")?;
    loop {
        let (_, terminal_height) = term_size::dimensions().unwrap_or((80, 24));
        let input_row = terminal_height;
        let history_rows = terminal_height - 1;
        print!("\x1B[1;{}r", history_rows);
        io::stdout().flush().unwrap();

        let mut input = String::new();

        print!("\x1B[{};1H\x1B[2K> ", input_row);
        io::stdout().flush().unwrap();

        io::stdin().read_line(&mut input).expect("no input");
        let trimmed = input.trim();
        if trimmed.eq("/exit") {
            break
        } else {
            print!(
                "\x1B[{};1H\x1B[1S\x1B[{};1H\x1B[2KYou: {}", 
                input_row, history_rows, trimmed
            );
            stream.write_all(trimmed.as_bytes())?;
            stream.write_all(b"\n")?;
 
            io::stdout().flush().unwrap();
        }
    }
    println!("break");
    Ok(())
}
