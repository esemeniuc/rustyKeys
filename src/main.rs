use std::collections::HashMap;

use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
mod tokenize;

const ERR_UNK_CMD: &str = "-ERR unknown command\r\n";
const NIL_MSG: &str = "$-1\r\n";

type TestRCMap<T, U> = Arc<RwLock<HashMap<T, U>>>;

async fn client_loop(mut stream: TcpStream, dict: TestRCMap<String, String>) -> io::Result<()> {
//    println!("Accepted from: {}", stream.peer_addr()?);
    let mut buf = [0u8; 1024]; //1MB
    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Ok(()); };
        let tokens = if buf[0] == '*' as u8 {
            tokenize::resp_tokenize(&buf[0..num_bytes])
        } else {
            tokenize::netcat_tokenize(&buf[0..num_bytes])
        };
        println!("tokens{:?}", tokens);
        let result = serve_request(&tokens, &dict).await;
        stream.write(result.as_ref()).await?;
    }
}

async fn serve_request(tokens: &Vec<&str>, dict: &TestRCMap<String, String>) -> String {
    if tokens.len() == 0 {
        return String::from(ERR_UNK_CMD);
    }
    match (tokens[0], tokens.len()) {
        ("GET", 2) => get_req(tokens[1], &dict).await,
        ("SET", 3) => set_req(tokens[1], tokens[2], &dict).await,
        ("SETNX", 3) => setnx_req(tokens[1], tokens[2], &dict).await,
        ("DEL", count) if count >= 1 => del_req(&tokens[1..], &dict).await,
        ("EXISTS", 2) => exists_req(tokens[1], &dict).await,
        ("TYPE", 2) => type_req(tokens[1], &dict).await,
        _ => String::from(ERR_UNK_CMD),
    }
}

//TODO:
//\item RENAME key newkey
//\item KEYS regex\_pattern


fn resp_bulk_format(actual_data: &str) -> String {
//A "$" byte followed by the number of bytes composing the string (a prefixed length), terminated by CRLF.
//The actual string data.
//A final CRLF.
//see https://redis.io/topics/protocol
    format!("${}\r\n{}\r\n", actual_data.len(), actual_data)
}

fn integer_format(x: isize) -> String {
    //see https://redis.io/topics/protocol
    format!(":{}\r\n", x)
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
        _ => format!("+OK\r\n"),
    }
}

async fn setnx_req(key: &str, val: &str, dict: &TestRCMap<String, String>) -> String {
//https://redis.io/commands/setnx
    let key = String::from(key);
    let val = String::from(val);
    let mut dict = dict.write().await;
    if (*dict).contains_key(&key) {
        return integer_format(0);
    }
    (*dict).insert(key, val);
    integer_format(1)
}

async fn exists_req(key: &str, dict: &TestRCMap<String, String>) -> String {
//https://redis.io/commands/exists
//TODO handle arbitrary number of keys
    let dict = dict.read().await;
    match (*dict).contains_key(key) {
        true => integer_format(1),
        false => integer_format(0)
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
    integer_format(count)
}

async fn type_req(key: &str, dict: &TestRCMap<String, String>) -> String {
//https://redis.io/commands/type
    let dict = dict.read().await;
    match (*dict).contains_key(key) {
        true => format!("+string\r\n"),
        false => format!("+none\r\n"),
    }
}

fn main() -> io::Result<()> {
    tokenize::test();
    let dict = Arc::new(RwLock::new(HashMap::new()));
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:6379").await?;
        println!("Listening on {}", listener.local_addr()?);
        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let dict = Arc::clone(&dict);
            task::spawn(async { client_loop(stream, dict).await.unwrap_or_default(); });
        }
        Ok(())
    })
}