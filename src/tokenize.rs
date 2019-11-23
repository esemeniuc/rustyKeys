use commands::tokenizer::{tokenize, TokenType};
use std::str::from_utf8;
const CRLF: &[u8] = b"\r\n";

pub(crate) fn test() {
    assert!(parse_int("1234\r\n".as_bytes()) == (1234, 4));
    assert!(parse_int("0\r\n".as_bytes()) == (0, 1));
    assert!(parse_int("\r\n".as_bytes()) == (0, 0));
    assert!(parse_int("1\r\n".as_bytes()) == (1, 1));
    assert!(parse_int("10391\r\n".as_bytes()) == (10391, 5));
    assert!(resp_tokenize("*2\r\n$10\r\nGETTTTTTAB\r\n$11\r\naaaaaaaaaxy\r\n".as_ref()) == vec!["GETTTTTTAB", "aaaaaaaaaxy"]);
    assert!(resp_tokenize("*2\r\n$3\r\nGET\r\n$5\r\napple\r\n".as_ref()) == vec!["GET", "apple"]);
    assert!(resp_tokenize("*3\r\n$3\r\nSET\r\n$2\r\nXX\r\n$5\r\napple\r\n".as_ref()) == vec!["SET", "XX", "apple"]);
    assert!(resp_tokenize("*4\r\n$3\r\nDEL\r\n$2\r\nXX\r\n$1\r\nY\r\n$3\r\nABC\r\n".as_ref()) == vec!["DEL", "XX", "Y", "ABC"]);
    assert!(resp_tokenize("*1\r\n$4\r\nPING\r\n".as_ref()) == vec!["PING"]);
}

//TODO handle non utf8 properly
pub(crate) fn netcat_tokenize(buf: &[u8]) -> Vec<&str> {
    let s = from_utf8(buf).unwrap_or_default().trim();
    let t = tokenize(s).unwrap_or_default();
    t.iter()
        .filter(|&x| x.token_type == TokenType::Word)
        .map(|&x| x.text).collect()
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

pub(crate) fn resp_tokenize(buf: &[u8]) -> Vec<&str> {
    /*eg. ran: cli.get('a')
    The client sends:
    *2\r\n      //token count
    $3\r\n      //first cmd length
    GET\r\n     //command
    $1\r\n      //next token len
    a\r\n       //token
    */

    macro_rules! consume_valid_separator_or_return {
    //consumes a $sep if in range, and updates i to point after $sep
    //else returns out
    //see https://medium.com/@phoomparin/a-beginners-guide-to-rust-macros-5c75594498f1
        ($buf: expr, $sep: expr, $i: expr, $retval: expr) => {
            if $i + $sep.len() >= $buf.len() ||
                &$buf[$i..$i + $sep.len()] != $sep {
                return $retval;
            } else {
                $i += $sep.len(); //point to beginning of rest
            }
        }
    }

    let mut out = Vec::new();
    let (token_count, iter_offset) = parse_int(&buf[1..]); //first char is asterisk
    let mut i = iter_offset + 1; //i points to next char after number

    consume_valid_separator_or_return!(buf, CRLF, i, out);
    for _ in 0..token_count {
        consume_valid_separator_or_return!(buf, b"$", i, out);
        let (cmd_len, iter_offset) = parse_int(&buf[i..]); //get command length
        i += iter_offset;
        consume_valid_separator_or_return!(buf, CRLF, i, out);

        //push token
        if let Ok(x) = from_utf8(&buf[i..i + cmd_len]) {
            out.push(x);
        }

        i += cmd_len;
        consume_valid_separator_or_return!(buf, CRLF, i, out);
    }
    out
}
