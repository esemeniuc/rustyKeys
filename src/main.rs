use std::collections::HashMap;

use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;

mod formatter;
mod req_handler;
mod tokenize;

type TestRCMap<T, U> = Arc<RwLock<HashMap<T, U>>>;

async fn client_loop(mut stream: TcpStream, dict: TestRCMap<String, String>) -> io::Result<()> {
    //    println!("Accepted from: {}", stream.peer_addr()?);
    let mut buf = [0u8; 1024]; //1MB
    loop {
        let num_bytes = stream.read(&mut buf).await?;
        if num_bytes <= 0 {
            return Ok(());
        };
        let tokens = if buf[0] == '*' as u8 {
            tokenize::resp_tokenize(&buf[0..num_bytes])
        } else {
            tokenize::netcat_tokenize(&buf[0..num_bytes])
        };
        println!("tokens{:?}", tokens);
        let result = req_handler::serve_request(&tokens, &dict).await;
        stream.write(result.as_ref()).await?;
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
            task::spawn(async {
                client_loop(stream, dict).await.unwrap_or_default();
            });
        }
        Ok(())
    })
}
