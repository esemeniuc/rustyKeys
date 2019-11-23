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
const NIL_MSG: &str = "$-1\r\n";
const CRLF: &[u8] = b"\r\n";

type TestRCMap<T, U> = Arc<RwLock<HashMap<T, U>>>;

fn test() {
    assert!(parse_int("1234\r\n".as_bytes()) == (1234, 4));
    assert!(parse_int("0\r\n".as_bytes()) == (0, 1));
    assert!(parse_int("\r\n".as_bytes()) == (0, 0));
    assert!(parse_int("1\r\n".as_bytes()) == (1, 1));
    assert!(parse_int("10391\r\n".as_bytes()) == (10391, 5));
    assert!(tokenize2("*2\r\n$10\r\nGETTTTTTAB\r\n$11\r\naaaaaaaaaxy\r\n".as_ref()) == vec!["GETTTTTTAB", "aaaaaaaaaxy"]);
    assert!(tokenize2("*2\r\n$3\r\nGET\r\n$5\r\napple\r\n".as_ref()) == vec!["GET", "apple"]);
    assert!(tokenize2("*3\r\n$3\r\nSET\r\n$2\r\nXX\r\n$5\r\napple\r\n".as_ref()) == vec!["SET", "XX", "apple"]);
    assert!(tokenize2("*4\r\n$3\r\nDEL\r\n$2\r\nXX\r\n$1\r\nY\r\n$3\r\nABC\r\n".as_ref()) == vec!["DEL", "XX", "Y", "ABC"]);
    assert!(tokenize2("*1\r\n$4\r\nPING\r\n".as_ref()) == vec!["PING"]);
}

//stops on non digit number
//returns index of last parsed digit
fn parse_int(buf: &[u8]) -> (usize, usize) {
    let mut i = 0;
    let mut out = 0usize;
    while i < buf.len() && buf[i].is_ascii_digit() {
        out *= 10;
        out += (buf[i] as char).to_digit(10).unwrap_or_default() as usize;
        i += 1;
    }
    (out, i)
}

//consumes a new line if in range, and updates i to point after newline
//else returns out
macro_rules! consume_valid_newline_or_return {
    ($buf: expr, $i: expr, $retval: expr) => {
        if $i + CRLF.len() >= $buf.len() ||
            &$buf[$i..$i + CRLF.len()] != CRLF {
            return $retval;
        } else {
            $i += CRLF.len(); //point to beginning of rest
        }
    }
}

fn tokenize2(buf: &[u8]) -> Vec<String> {
    /*eg. ran: cli.get('a')
    The client sends:
    *2\r\n //token count
    $3\r\n //first cmd length
    GET\r\n //command
    $1\r\n //next token len
    a\r\n*/ //token
    let mut out = Vec::new();
    let (token_count, iter_offset) = parse_int(&buf[1..]); //first char is asterisk
    let mut i = iter_offset + 1; //i points to next char after number

    consume_valid_newline_or_return!(buf, i, out);
    for _ in 0..token_count {
        if buf[i] != '$' as u8 { return out; } else { i += 1; }
        let (cmd_len, iter_offset) = parse_int(&buf[i..]); //get command length
        i += iter_offset;
        consume_valid_newline_or_return!(buf, i, out);

        //push token
        if let Ok(x) = from_utf8(&buf[i..i + cmd_len]) {
            out.push(String::from(x));
        }

        i += cmd_len;
        consume_valid_newline_or_return!(buf, i, out);
    }
    out
}

async fn process(mut stream: TcpStream, dict: TestRCMap<String, String>) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);
    let mut buf = [0u8; 1024]; //1MB

    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 { return Err(Error::new(ErrorKind::ConnectionAborted, "no bytes")); };
        if buf[0] == '*' as u8 {
            let tokens = tokenize2(&buf);
            println!("tokens{:?}", tokens);
        } else { //handle netcat
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
                    ("TYPE", 2) => stream.write(type_req(&tokens[1], &dict).await.as_ref()).await?,
                    _ => stream.write(ERR_UNK_CMD).await?,
                };
            } else {
                stream.write(ERR_UNK_CMD).await?;
            }
        }
    }
}


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
    format!("${}\r\n{}\r\n", actual_data.len(), actual_data)
}

fn integer_format(x: isize) -> String {
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
    test();
    let dict = Arc::new(RwLock::new(HashMap::new()));
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:6379").await?;
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