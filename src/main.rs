use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use std::io::{Error, ErrorKind};
use std::str::from_utf8;
use async_std::sync::{Arc, RwLock};
use std::collections::HashMap;
use commands::tokenizer::tokenize;

const ERR_UNK_CMD: &[u8] = b"-ERR unknown command\r\n";
const NIL_MSG: &str = "$-1\n";

type TestRCMap<T, U> = Arc<RwLock<HashMap<T, U>>>;

async fn process(mut stream: TcpStream, dict: TestRCMap<String, String>) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);
    stream.write(b"Welcome\n").await?;
    let mut buf = [0; 1 << 10]; //1MB

    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Err(Error::new(ErrorKind::ConnectionAborted, "no bytes")); };
        let s = from_utf8(&buf[0..num_bytes]).unwrap().trim(); //TODO handle UTF8
        if let Ok(tokens) = tokenize(s) {
            if tokens.len() == 0 {
                stream.write(ERR_UNK_CMD).await?;
            }
            match (tokens[0].text, tokens.len()) {
                ("GET", 3) => stream.write(get_req(tokens[2].text, &dict).await.as_ref()).await?,
                ("SET", 5) => stream.write(set_req(tokens[2].text, tokens[4].text, &dict).await.as_ref()).await?,
                ("DEL", 3) => stream.write(del_req(vec![tokens[2].text], &dict).await.as_ref()).await?, //TODO handle any number of deletions
                _ => stream.write(ERR_UNK_CMD).await?,
            };
        } else {
            stream.write(ERR_UNK_CMD).await?;
        }
    }
}

//\item SETNX key value
//\item EXISTS key
//\item TYPE key
//\item RENAME key newkey
//\item KEYS regex\_pattern

fn resp_bulk_format(actual_data: &String) -> String {
    //A "$" byte followed by the number of bytes composing the string (a prefixed length), terminated by CRLF.
    //The actual string data.
    //A final CRLF.
    //see https://redis.io/topics/protocol
    format!("${}\n{}\n", actual_data.len(), actual_data)
}

async fn get_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    let dict = dict.read().await;
    match (*dict).get(key) {
        Some(val) => resp_bulk_format(val),
        None => String::from(NIL_MSG),
    }
}

async fn set_req(key: &str, val: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/SET
    //TODO Handle NX and XX
    let mut dict = dict.write().await;
    match (*dict).insert(String::from(key), String::from(val)) {
        _ => format!("+OK\n"),
    }
}


async fn del_req(keys: Vec<&str>, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/del
    let mut dict = dict.write().await;
    let mut count: isize = 0;
    for key in keys {
        match (*dict).remove(key) {
            Some(_) => count += 1,
            _ => {}
        }
    }
    format!(":{}\n", count)
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