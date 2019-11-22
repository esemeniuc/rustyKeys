use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;
use std::io::{Error, ErrorKind};
use std::str::from_utf8;
use async_std::sync::{Arc, RwLock};
use std::collections::HashMap;
use commands::tokenizer::{Token, tokenize, TokenType};

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
            let tokens = get_tokens(tokens);
            match (tokens[0], tokens.len()) {
                ("GET", 2) => stream.write(get_req(tokens[1], &dict).await.as_ref()).await?,
                ("SET", 3) => stream.write(set_req(tokens[1], tokens[2], &dict).await.as_ref()).await?,
                ("SETNX", 3) => stream.write(setnx_req(tokens[1], tokens[2], &dict).await.as_ref()).await?,
                ("DEL", count) if count >= 1 => stream.write(del_req(&tokens[1..], &dict).await.as_ref()).await?,
                ("EXISTS", 2) => stream.write(exists_req(&tokens[1], &dict).await.as_ref()).await?,
                _ => stream.write(ERR_UNK_CMD).await?,
            };
        } else {
            stream.write(ERR_UNK_CMD).await?;
        }
    }
}

//\item TYPE key
//\item RENAME key newkey
//\item KEYS regex\_pattern

fn get_tokens(tokens: Vec<Token>) -> Vec<&str> {
    tokens.iter()
        .filter(|&x| x.token_type == TokenType::Word)
        .map(|&x| x.text).collect()
}

fn resp_bulk_format(actual_data: &str) -> String {
    //A "$" byte followed by the number of bytes composing the string (a prefixed length), terminated by CRLF.
    //The actual string data.
    //A final CRLF.
    //see https://redis.io/topics/protocol
    format!("${}\n{}\n", actual_data.len(), actual_data)
}

async fn get_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/GET
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

async fn setnx_req(key: &str, val: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/setnx
    let key = String::from(key);
    let val = String::from(val);
    let mut dict = dict.write().await;
    if (*dict).contains_key(&key) {
        return format!(":0\n");
    }
    (*dict).insert(key, val);
    format!(":1\n")
}

async fn exists_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/exists
    //TODO handle arbitrary number of keys
    let dict = dict.read().await;
    match (*dict).contains_key(key) {
        true => format!(":1\n"),
        false => format!(":0\n"),
    }
}

async fn del_req(keys: &[&str], dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/del
    let mut count: isize = 0;
    let mut dict = dict.write().await;
    for key in keys {
        match (*dict).remove(*key) {
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