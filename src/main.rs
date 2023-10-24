use postgres::{Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
struct Pet {
    id: Option<i32>,
    name: String,
    breed: String,
}

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";
fn main() {
    let db_url: &str = option_env!("DB_URL").unwrap();
    let server_address: &str = option_env!("SERVER_ADDR").unwrap();

    if let Err(e) = set_database(db_url) {
        println!("Error: {}", e);
        return;
    }

    let listener = TcpListener::bind(server_address).unwrap();
    println!("Listening on {}", server_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream, db_url);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn set_database(db_url: &str) -> Result<(), PostgresError> {
    let mut client = Client::connect(db_url, NoTls)?;

    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS dogs (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            weight INTEGER NOT NULL,
            breed VARCHAR NOT NULL,
            apt INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS cats (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            weight INTEGER NOT NULL,
            hair VARCHAR NOT NULL,
            apt INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS birds (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            species VARCHAR NOT NULL,
            apt INTEGER NOT NULL
        );
        CREATE INDEX dogs_apt_idx ON dogs(apt);
        CREATE INDEX cats_apt_idx ON cats(apt);
        CREATE INDEX birds_apt_idx ON birds(apt);"
    )?;
    Ok(())
}

fn get_id(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

fn get_pet_request_body(request: &str) -> Result<Pet, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}

fn handle_client(mut stream: TcpStream, db_url: &str) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[0..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /pet") => handle_post_request(r, db_url),
                r if r.starts_with("GET /pet/") => handle_get_request(r, db_url),
                _ => (NOT_FOUND.to_string(), "404 NOT FOUND".to_string()),
            };

            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

fn handle_post_request(request: &str, db_url: &str) -> (String, String) {
    println!("Received POST request: {}", request);
    match (get_pet_request_body(&request), Client::connect(db_url, NoTls)) {
        (Ok(pet), Ok(mut client)) => {
            client
            .execute(
                "INSERT INTO pets (name, breed) VALUES ($1, $2)",
                &[&pet.name, &pet.breed]
            )
            .unwrap();

            (OK_RESPONSE.to_string(), "Pet created".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Internal server error".to_string())
    }
}

fn handle_get_request(request: &str, db_url: &str) -> (String, String) {
    println!("Received GET request: {}", request);
    match (get_id(&request).parse::<i32>(), Client::connect(db_url, NoTls)) {
        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM pets WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let pet = Pet {
                        id: row.get(0),
                        name: row.get(1),
                        breed: row.get(2),
                    };

                    (OK_RESPONSE.to_string(), serde_json::to_string(&pet).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "Pet not found".to_string()),
            }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}
