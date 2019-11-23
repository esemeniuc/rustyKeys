use crate::formatter;
use crate::TestRCMap;

//TODO:
//\item RENAME key newkey
//\item KEYS regex\_pattern
const NIL_MSG: &str = "$-1\r\n";
const ERR_UNK_CMD: &str = "-ERR unknown command\r\n";

pub(crate) async fn serve_request(tokens: &Vec<&str>, dict: &TestRCMap<String, String>) -> String {
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


async fn get_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/GET
    let dict = dict.read().await;
    match (*dict).get(key) {
        Some(val) => formatter::resp_bulk_format(val),
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
        return formatter::integer_format(0);
    }
    (*dict).insert(key, val);
    formatter::integer_format(1)
}

async fn exists_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/exists
    //TODO handle arbitrary number of keys
    let dict = dict.read().await;
    match (*dict).contains_key(key) {
        true => formatter::integer_format(1),
        false => formatter::integer_format(0),
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
    formatter::integer_format(count)
}

async fn type_req(key: &str, dict: &TestRCMap<String, String>) -> String {
    //https://redis.io/commands/type
    let dict = dict.read().await;
    match (*dict).contains_key(key) {
        true => format!("+string\r\n"),
        false => format!("+none\r\n"),
    }
}
