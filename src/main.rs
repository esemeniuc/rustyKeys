use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use std::io::{Error, ErrorKind};
use std::str::from_utf8;
//use std::sync::RwLock;

const ERROR_MSG: &[u8] = b"Error: Invalid command\r\n";

async fn process(mut stream: TcpStream) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);

    stream.write(b"Welcome\n").await?;
    print!("got connection!\n");
    let mut buf = [0; 1024]; //1KB

    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Err(Error::new(ErrorKind::ConnectionAborted, "no bytes")); };
        let s = from_utf8(&buf[0..num_bytes]).unwrap();
//        println!("Got {} bytes, msg: {}", num_bytes, s);

        match s.find(' ') {
            Some(len) => match &s[..len] {
                "GET" => stream.write(get_req(&s[len..]).as_ref()).await?,
                "SET" => stream.write(b"set it").await?,
                _ => stream.write(ERROR_MSG).await?
            },
            None => stream.write(ERROR_MSG).await?
        };
    };
}

fn get_req(key: &str) -> String {
    let val = 5;
    format!("got {}", val)
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            task::spawn(async {
                process(stream).await.unwrap();
            });
        }
        Ok(())
    })
}