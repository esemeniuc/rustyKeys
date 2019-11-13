use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use std::io::{Error, ErrorKind};
use std::str::from_utf8;
use async_std::sync::{Arc, RwLock};
use std::collections::HashMap;

const ERROR_MSG: &[u8] = b"Error: Invalid command\r\n";

type TestRCMap<T, U> = Arc<RwLock<HashMap<T, U>>>;

async fn process(mut stream: TcpStream, dict: TestRCMap<String, String>) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);
    stream.write(b"Welcome\n").await?;
    let mut buf = [0; 1024]; //1KB

    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Err(Error::new(ErrorKind::ConnectionAborted, "no bytes")); };
        let s = from_utf8(&buf[0..num_bytes]).unwrap().trim(); //TODO handle UTF8
//        println!("Got {} bytes, msg: {}", num_bytes, s);

        match s.find(' ') {
            Some(len) => match &s[..len] {
                "GET" => stream.write(get_req(&s[(1 + len)..], &dict).await.as_ref()).await?,
                "SET" => match &s[(len + 1)..].find(' ') {
                    Some(len2) => 
                        stream.write(
                            set_req(&s[(1 + len)..(1 + len + *len2)], 
                                    &s[(2 + len + *len2)..], &dict).await.as_ref()).await?,
                    None => stream.write(ERROR_MSG).await?
                },
                "DEL" => stream.write(del_req(&s[(1 + len)..], &dict).await.as_ref()).await?,
                _ => stream.write(ERROR_MSG).await?,
            },
            None => stream.write(ERROR_MSG).await?
        };
    };
}

async fn get_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    let dict = dict.read().await;
    match (*dict).get(key) {
        Some(val) => format!("got {}\n", val),
        None => format!("Error, key ${}$ not found\n", key),
    }
}

async fn set_req(key: &str, val: &str, dict: &TestRCMap<String, String>) -> String {
    let mut dict = dict.write().await;
    match (*dict).insert(String::from(key), String::from(val)) {
        Some(val) => format!("set successful, old value {}\n", val), //TODO, make correct response via Redis spec
        None => format!("set successful with new key {}\n", key),
    }
}

async fn del_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    let mut dict = dict.write().await;
    match (*dict).remove(key) {
        Some(val) => format!("delete successful, removed {}\n", val),
        None => format!("Error, key ${}$ not found\n", key),
    }
}

fn main() -> io::Result<()> {
    let dict = Arc::new(RwLock::new(HashMap::new()));
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);
        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let dict = Arc::clone(&dict);
            task::spawn(async { process(stream, dict).await.unwrap(); });
        }
        Ok(())
    })
}