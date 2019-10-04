use std::net::{TcpListener, TcpStream};
use std::io::{Write, Read, Error};

fn handle_client(mut stream: TcpStream) -> Result<(), Error> {
    stream.write(b"Welcome\n")?;
    print!("got connection!\n");

    let mut buf = [0; 5]; //5 bytes
    loop {
        let num_bytes = stream.read(&mut buf);
        match num_bytes {
            Ok(0) => { return Ok(()); }
            Ok(num_bytes) => {
                println!("Got {} bytes", num_bytes);
                stream.write(&buf[0..num_bytes])?;
            }
            Err(e) => { return Err(e); }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    print!("Listening\n");

    // accept connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => { let _ = handle_client(stream); }
            Err(e) => { println!("Error in handler: {}", e); }
        }
    }
}