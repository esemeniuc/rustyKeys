use std::net::{TcpListener, TcpStream};
use std::io::{Write, Read};
use futures::executor::ThreadPool;

async fn handle_client(mut stream: TcpStream) {
    stream.write(b"Welcome\n").expect("Error writing");
    print!("got connection!\n");
    let mut buf = [0; 1024]; //1KB

    loop {
        let num_bytes = stream.read(&mut buf).expect("Error reading");
        println!("Got {} bytes", num_bytes);
        stream.write(&buf[0..num_bytes]).expect("Error writing");
    }
}

fn main() {
    let pool = ThreadPool::new().expect("Failed to create threadpool");
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    print!("Listening\n");

    // accept connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => { pool.spawn_ok(handle_client(stream)); }
            Err(e) => { println!("Error receiving incoming connection: {}", e); }
        }
    }
}