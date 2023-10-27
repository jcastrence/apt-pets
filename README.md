# Apartment Pets REST API
---
### Running Application

1. Build and Run Postgres DB
```docker compose up -d db```
2. Build Rust Binaries
```docker compose build```
3. Run Rust App
```docker compose up app```
---
### Endpoints

1. Example Post Request: [ip:port]/pets/[apartment number]
```
curl -X POST \
--location 'http://0.0.0.0:8080/pets/123' \
--header 'Content-Type: application/json' \
--data '[
    {
        "animal": "Dog",
        "name": "Sunny",
        "weight": 70,
        "breed": "Labrador"
    },
    {
        "animal": "Dog",
        "name": "Paris",
        "weight": 60,
        "breed": "Poodle"
    },
    {
        "animal": "Cat",
        "name": "Nova",
        "weight": 13,
        "hair": "LongHaired"
    },
    {
        "animal": "Cat",
        "name": "Fenrir",
        "weight": 7,
        "hair": "ShortHaired"
    },
    {
        "animal": "Bird",
        "name": "Polly",
        "species": "Parrot"
    }
]'
```

2. Example Get Request: [ip:port]/pets/[apartment number]
```
curl -X GET \
--location 'http://0.0.0.0:8080/pets/123' \
--header 'Content-Type: application/json'
```