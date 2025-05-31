# Zephyr Data Provider

Data provider for https://cat.zephyrapp.nz

## API

API consists of the endpoints that reply to GET method.

Following endpoints return plain text:
- `/api/v1/health`
- `/api/v1/version`

Following endpoints return JSON data:
- `/api/v1/units?token=API_TOKEN`
- `/api/v1/stations?token=API_TOKEN`
- `/api/v1/measurements?token=API_TOKEN`

Responses are UTF-8 encoded. JSON data isn't sorted.

Objects in `/units` and `/stations` have fixed structure where all the fields are mandatory.

`/measurements` collect only data from those stations that provide `wind_speed` and `wind_direction`. `gusts_speed` and `temperature` are optional and will be nulled if readings are not available. When `wind_speed` is 0, `wind_direction` is considered unreliable and will be nulled. 

## Development

```elvish
cargo fmt
cargo clippy
set-env SPIN_VARIABLE_API_TOKEN (tr -dc A-Za-z0-9 </dev/urandom | head -c 16)
spin up --build
```

## Deployment

```elvish
spin deploy --build --variable api_token=$E:SPIN_VARIABLE_API_TOKEN
```
