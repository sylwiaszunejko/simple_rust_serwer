use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use std::str;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use colored::Colorize;

type Db = Arc<Mutex<HashMap<String, String>>>;

#[tokio::main]
async fn main() {
    // Listen for incoming TCP connections on localhost port 5555
    let listener = match TcpListener::bind("0.0.0.0:5555").await {
        Ok(n) => {
            println!("{}", "Server up and running... ".green());
            n
        }
        Err(e) => {
            eprintln!(
                "{}\n{}",
                "Failed to start server, reason:".red(),
                e.to_string()
            );
            return;
        }
    };

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone();

        tokio::spawn(async move {
            handle_connection(socket, db).await;
        });
    }
}

async fn handle_connection(socket: TcpStream, db: Db) {
    let (mut rd, mut wr) = io::split(socket);
    
    let store = b"STORE";
    let load = b"LOAD$";

    loop {
        let mut header = vec![0; 5];
        let mut separator = vec![0; 1];

        match rd.read_exact(&mut header).await {
            Ok(_) => (),
            Err(_) => return,
        }

        if header == load {
            let mut key = Vec::new();
            let mut buff = vec![0; 1];

            while rd.read_exact(&mut buff).await.unwrap() == 1 && buff != b"$" {
                key.push(buff[0]);
            }

            let mut value = String::from("not_found");
            match db.lock() {
                Ok(db) => {
                    if db.contains_key(&String::from(str::from_utf8(&key).unwrap())) {
                        value = String::from(db.get(str::from_utf8(&key).unwrap()).unwrap().as_str());
                    }
                },
                Err(_) => return,
            }

            if value != "not_found" {
                match wr.write_all(format!("FOUND${}$", value).as_bytes()).await {
                    Ok(_) => (),
                    Err(_) => return,
                }
            } else {
                match wr.write_all(b"NOTFOUND$").await {
                    Ok(_) => (),
                    Err(_) => return,
                }
            }
        } else if header == store {
            match rd.read_exact(&mut separator).await {
                Ok(_) => {
                    if separator != b"$" {break;}
                    let mut key = Vec::new();
                    let mut buff = vec![0; 1];

                    while rd.read_exact(&mut buff).await.is_ok() && buff != b"$" {
                        key.push(buff[0]);
                    }

                    let mut value = Vec::new();
                    while rd.read_exact(&mut buff).await.is_ok() && buff != b"$" {
                        value.push(buff[0]);
                    }

                    match db.lock() {
                        Ok(mut db) => {
                            db.insert(String::from(str::from_utf8(&key[..]).unwrap()), String::from(str::from_utf8(&value[..]).unwrap()));
                    
                        },
                        Err(_) => return,
                    }
                    
                    match wr.write_all(b"DONE$").await {
                        Ok(_) => (),
                        Err(_) => return,
                    }
                },
                Err(_) => return,
            }
        }
    }

}
