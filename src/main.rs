use postgres::{Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
struct Dog {
    name: String,
    weight: i32,
    breed: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Cat {
    name: String,
    weight: i32,
    hair: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Bird {
    name: String,
    species: String,
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
        "CREATE TABLE IF NOT EXISTS apts (
            id SERIAL PRIMARY KEY,
            apt INTEGER NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS dogs (
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
            hair BOOLEAN NOT NULL,
            apt INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS birds (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            species VARCHAR NOT NULL,
            apt INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS apt_idx ON apts USING HASH(apt);
        CREATE INDEX IF NOT EXISTS dogs_apt_idx ON dogs(apt);
        CREATE INDEX IF NOT EXISTS cats_apt_idx ON cats(apt);
        CREATE INDEX IF NOT EXISTS birds_apt_idx ON birds(apt);"
    )?;
    Ok(())
}

fn get_apt(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

fn get_request_body(request: &str) -> Result<serde_json::Value, serde_json::Error> {
    println!("Request: {}", request);
    let res = serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default());
    println!("Result: {:?}", res);
    res
}

fn handle_client(mut stream: TcpStream, db_url: &str) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[0..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /pets/") => handle_post_request(r, db_url),
                r if r.starts_with("GET /pets/") => handle_get_request(r, db_url),
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
    match (get_apt(&request).parse::<i32>(), Client::connect(db_url, NoTls)) {
        (Ok(apt), Ok(mut client)) =>
            match client.query("SELECT * FROM apts WHERE apt = $1", &[&apt]) {
                Ok(rows) => {
                    match rows.len() {
                        0 => {
                            match get_request_body(&request) {
                                Ok(body) => {
                                    match get_pets_vecs(body) {
                                        Ok(pets) => {
                                            println!("Pets: {:?}", pets);
                                            client.execute("INSERT INTO apts (apt) VALUES ($1)", &[&apt]).unwrap();

                                            if pets.0.len() > 0 {
                                                for dog in pets.0 {
                                                    client.execute(
                                                        "INSERT INTO dogs (name, weight, breed, apt)
                                                            VALUES ($1, $2, $3, $4)",
                                                        &[&dog.name, &dog.weight, &dog.breed, &apt]
                                                    ).unwrap();
                                                }
                                            }

                                            if pets.1.len() > 0 {
                                                for cat in pets.1 {
                                                    client.execute(
                                                        "INSERT INTO cats (name, weight, hair, apt)
                                                            VALUES ($1, $2, $3, $4)",
                                                        &[&cat.name, &cat.weight, &cat.hair, &apt]
                                                    ).unwrap();
                                                }
                                            }

                                            if pets.2.len() > 0 {
                                                for bird in pets.2 {
                                                    client.execute(
                                                        "INSERT INTO birds (name, species, apt)
                                                            VALUES ($1, $2, $3)",
                                                        &[&bird.name, &bird.species, &apt]
                                                    ).unwrap();
                                                }
                                            }

                                            (OK_RESPONSE.to_string(), "Pets created".to_string())
                                        },
                                        Err(e) => (INTERNAL_SERVER_ERROR.to_string(), e)
                                    }
                                },
                                _ => (INTERNAL_SERVER_ERROR.to_string(), "Bad json body".to_string())
                            }
                        },
                        1 => (INTERNAL_SERVER_ERROR.to_string(), "Pets already registered to this apartment".to_string()),
                        _ => (INTERNAL_SERVER_ERROR.to_string(), "Unexpected query value".to_string()) // Should never happen since apt numbers are unique
                    }
                },
                Err(e) => (INTERNAL_SERVER_ERROR.to_string(), e.to_string())
            },
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error: Bad apartment".to_string())
    }
}

fn handle_get_request(request: &str, db_url: &str) -> (String, String) {
    println!("Received GET request: {}", request);
    match (get_apt(&request).parse::<i32>(), Client::connect(db_url, NoTls)) {
        (Ok(apt), Ok(mut client)) =>
            match client.query_one("SELECT * FROM dogs WHERE apt = $1", &[&apt]) {
                Ok(row) => {
                    let dog = Dog {
                        name: row.get("name"),
                        weight: row.get("weight"),
                        breed: row.get("breed")
                    };

                    (OK_RESPONSE.to_string(), serde_json::to_string(&dog).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "Pet not found".to_string()),
            }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn get_pets_vecs(a: serde_json::Value) -> Result<(Vec<Dog>, Vec<Cat>, Vec<Bird>), String> {
    match a {
        serde_json::Value::Array(a) => {
            let mut dogs: Vec<Dog> = Vec::new();
            let mut cats: Vec<Cat> = Vec::new();
            let mut birds: Vec<Bird> = Vec::new();
            for v in a {
                match v {
                    serde_json::Value::Object(pet) => {
                        match pet.get("animal") {
                            Some(animal) => {
                                match animal.as_str() {
                                    Some("Dog") => {
                                        let name: &str;
                                        let weight: i32;
                                        let breed: &str;
                                        match pet.get("name") {
                                            Some(n) => {
                                                match n.as_str() {
                                                    Some(n) => name = n,
                                                    None => return Err("Name field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Dogs require name field".to_string())
                                        };
                                        match pet.get("weight") {
                                            Some(w) => {
                                                match w.as_i64() {
                                                    Some(i) => weight = i as i32,
                                                    None => return Err("Weight field must be integer".to_string())
                                                }
                                            },
                                            None => return Err("Dogs require weight field".to_string())
                                        };
                                        match pet.get("breed") {
                                            Some(b) => { 
                                                match b.as_str() {
                                                    Some(b) => breed = b,
                                                    None => return Err("Breed field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Dogs require breed field".to_string())
                                        };
                                        dogs.push(Dog {
                                            name: name.to_string(),
                                            weight: weight,
                                            breed: breed.to_string(),
                                        });
                                    },
                                    Some("Cat") => {
                                        let name: &str;
                                        let weight: i32;
                                        let hair: bool;
                                        match pet.get("name") {
                                            Some(n) => {
                                                match n.as_str() {
                                                    Some(n) => name = n,
                                                    None => return Err("Name field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Cats require name field".to_string())
                                        };
                                        match pet.get("weight") {
                                            Some(w) => {
                                                match w.as_i64() {
                                                    Some(i) => weight = i as i32,
                                                    None => return Err("Weight field must be integer".to_string())
                                                }
                                            },
                                            None => return Err("Cats require weight field".to_string())
                                        };
                                        match pet.get("hair") {
                                            Some(h) => { 
                                                match h.as_str() {
                                                    Some(h) => {
                                                        match h {
                                                            "LongHaired" => hair = true,
                                                            "ShortHaired" => hair = false,
                                                            _ => return Err("Hair field must be either LongHaired or ShortHaired".to_string())
                                                        }
                                                    },
                                                    None => return Err("Hair field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Cats require hair field".to_string())
                                        };
                                        cats.push(Cat {
                                            name: name.to_string(),
                                            weight: weight,
                                            hair: hair,
                                        });
                                    },
                                    Some("Bird") => {
                                        let name: &str;
                                        let species: &str;
                                        match pet.get("name") {
                                            Some(n) => {
                                                match n.as_str() {
                                                    Some(n) => name = n,
                                                    None => return Err("Name field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Birds require name field".to_string())
                                        };
                                        match pet.get("species") {
                                            Some(s) => { 
                                                match s.as_str() {
                                                    Some(s) => species = s,
                                                    None => return Err("Species field must be string".to_string())
                                                }
                                            },
                                            None => return Err("Birds require species field".to_string())
                                        };
                                        birds.push(Bird {
                                            name: name.to_string(),
                                            species: species.to_string(),
                                        });
                                    },
                                    _ => return Err("Invalid pet type".to_string())
                                }
                            },
                            None => return Err("Each pet requires animal field".to_string())
                        } 
                    },
                    _ => return Err("Object is not pet".to_string())
                }
            }

            Ok((dogs, cats, birds))
        },
        _ => Err("Json must be array".to_string())
    }
}