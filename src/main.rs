use tokio::io::BufReader;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use std::collections::HashMap;
use std::io::Error;
use std::sync::{Arc, Mutex};
use colored::Colorize;
use regex::Regex;

type Db = Arc<Mutex<HashMap<String, String>>>;

// Error returned when something goes wrong during a task's work.
// We do not care what really happened because in every case we just
// finish the task and close the connection with the client.
#[derive(Debug)]
pub struct TaskError;

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
            match handle_connection(socket, db).await {
                Ok(_) => (),
                Err(_) => ()
            };
        });
    }
}

async fn handle_connection(socket: TcpStream, db: Db) -> Result<(), Error> {
    let (rd, mut wr) = io::split(socket);
    
    let store = b"STORE";
    let load = b"LOAD$";

    let mut buf_reader = BufReader::new(rd);

    loop {
        let mut header = vec![0; 5];
        let mut separator = vec![0; 1];

        buf_reader.read_exact(&mut header).await?;

        if header == load {
            let mut key = String::new();
            let mut buff = vec![0; 1];

            while buf_reader.read_exact(&mut buff).await.unwrap() == 1 && buff != b"$" {
                key.push(buff[0] as char);
            }

            let mut value = None;

            if !is_valid(key.clone()) {break;}
        
            if db.lock().unwrap().contains_key(&key) {
                value = Some(String::from(db.lock().unwrap().get(&key).unwrap()));
            }

            if value != None {
                wr.write_all(format!("FOUND${}$", value.unwrap()).as_bytes()).await?;
            } else {
                wr.write_all(b"NOTFOUND$").await?
            }
        } else if header == store {
            buf_reader.read_exact(&mut separator).await?;
            if separator != b"$" {break;}
            let mut key = String::new();
            let mut buff = vec![0; 1];

            while buf_reader.read_exact(&mut buff).await.is_ok() && buff != b"$" {
                key.push(buff[0] as char);
            }

            let mut value = String::new();
            while buf_reader.read_exact(&mut buff).await.is_ok() && buff != b"$" {
                value.push(buff[0] as char);
            }

            if !is_valid(key.clone()) {break;}
            if !is_valid(key.clone()) {break;}

            db.lock().unwrap().insert(key, value);
            
            wr.write_all(b"DONE$").await?;
        }
    }
    Ok(())
}

fn is_valid(s: String) -> bool {
    match Regex::new(r"^[a-z]*") {
        Ok(store_regex) => store_regex.is_match(s.as_str()),
        Err(_) => false
    }
}
