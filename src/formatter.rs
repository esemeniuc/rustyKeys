pub(crate) fn resp_bulk_format(actual_data: &str) -> String {
    //A "$" byte followed by the number of bytes composing the string (a prefixed length), terminated by CRLF.
    //The actual string data.
    //A final CRLF.
    //see https://redis.io/topics/protocol
    format!("${}\r\n{}\r\n", actual_data.len(), actual_data)
}

pub(crate) fn integer_format(x: isize) -> String {
    //see https://redis.io/topics/protocol
    format!(":{}\r\n", x)
}
