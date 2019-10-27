use std::net::{TcpListener, TcpStream};
use std::io::{Write, Read};
use futures::executor::ThreadPool;
use std::str::from_utf8;
use std::sync::RwLock;
use futures::task::SpawnExt;
use std::thread::sleep;
use std::time::Duration;

const ERROR_MSG: &[u8] = b"Error: Invalid command\r\n";

async fn handle_client(mut stream: TcpStream) {
    stream.write(b"Welcome\n").unwrap();
    print!("got connection!\n");
    let mut buf = [0; 1024]; //1KB

    loop {
        let num_bytes = stream.read(&mut buf).unwrap();
        if num_bytes <= 0 { return; }
        let s = from_utf8(&buf[0..num_bytes]).unwrap();
//        println!("Got {} bytes, msg: {}", num_bytes, s);

        s.find(' ').and_then(|len| match &s[..len] {
            "GET" => getReq(&mut stream, &s[len..]),
            "SET" => Some(stream.write(b"set it").unwrap()),
            _ => {
                stream.write(ERROR_MSG).unwrap();
                None
            }
        });
    };
}

fn getReq(stream: &mut TcpStream, key: &str) -> Option<usize> {
    let val = 5;
    sleep(Duration::from_millis(500));
    return Some(stream.write(format!("got {}", val).as_ref()).unwrap());
}

fn main() {
    let pool = ThreadPool::builder().pool_size(1).create().unwrap();
//    let pool = ThreadPool::new().expect("Failed to create threadpool");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    print!("Listening\n");

    // accept connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => pool.spawn_ok(handle_client(stream)),
            Err(e) => println!("Error receiving incoming connection: {}", e)
        }
    }
}