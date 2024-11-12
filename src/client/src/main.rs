use std::{
    io::{prelude::*},
    net::TcpStream,
};

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();

    let mut message = String::new();
    let mut buffer = [0; 1024];
    loop {
        message = "".to_string();
        println!("Enter your message to send:");
        std::io::stdin()
            .read_line(&mut message)
            .expect("Failed to read message");
            
        let message = message.trim();

        stream.write(message.as_bytes());
        stream.flush();
    }

    Ok(())
}
