use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use std::io::{Error, ErrorKind};
use std::str::from_utf8;
use async_std::sync::{Arc, Mutex};

//use std::sync::RwLock;
const ERROR_MSG: &[u8] = b"Error: Invalid command\r\n";

async fn process(mut stream: TcpStream, m: Arc<Mutex<i32>>) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);

    stream.write(b"Welcome\n").await?;
    print!("got connection!\n");
    let mut buf = [0; 1024]; //1KB

    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Err(Error::new(ErrorKind::ConnectionAborted, "no bytes")); };
        let s = from_utf8(&buf[0..num_bytes]).unwrap();
        let s1 = from_utf8(&buf[0..num_bytes]).unwrap();
//        println!("Got {} bytes, msg: {}", num_bytes, s);

        match s.find(' ') {
            Some(len) => match &s[..len] {
                "GET" => stream.write(get_req(&s[len..], &m).await.as_ref()).await?,
                "SET" => stream.write(b"set it").await?,
//                "SET" => stream.write(set_req(&s[len..], &s1[len..],&m).await.as_ref()).await?,
                _ => stream.write(ERROR_MSG).await?
            },
            None => stream.write(ERROR_MSG).await?
        };
    };
}

async fn get_req(key: &str, m: &Arc<Mutex<i32>>) -> String {
    let mut num = m.lock().await;
    (*num) = *num + 1;
    format!("got {}\n", *num)
}

async fn set_req(key: &str, val: &str, m: &Arc<Mutex<i32>>) -> String {
    let mut num = m.lock().await;
    (*num) = *num + 1;
    format!("set {}\n", *num)
}

fn main() -> io::Result<()> {
    let counter = Arc::new(Mutex::new(0));
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let counter = Arc::clone(&counter);
            task::spawn(async {
                process(stream, counter).await.unwrap();
            });
        }
        Ok(())
    })
}