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

Sample responses:

`/measurements`
```json
[
  {
    "station_id": "9648493fa0e6957dbe03eac2b18d1589",
    "wind_speed": 9,
    "wind_direction": 157.5,
    "gusts_speed": null,
    "temperature": 23.6,
    "last_update": "2025-06-05T09:39Z"
  },
  {
    "station_id": "caf0df10c3aa2e869fcaaf70707b78df",
    "wind_speed": 0,
    "wind_direction": null,
    "gusts_speed": null,
    "temperature": 22.8,
    "last_update": "2025-06-05T09:48Z"
  },
  ...
]
```

`/stations`
```json
[
  {
    "id": "f1452cf50d722bd73407d6f6b6535172",
    "name": "Badalona - Bufalà",
    "elevation": 65,
    "url": "https://www.meteoclimatic.net/perfil/ESCAT0800000008915A",
    "lat": 41.4596286927027,
    "long": 2.24343379705031
  },
  {
    "id": "6730960d7aa16fc5c1b61d3004170caa",
    "name": "Cornella de Llobregat",
    "elevation": 47,
    "url": "https://www.meteoclimatic.net/perfil/ESCAT0800000008940C",
    "lat": 41.3563454446431,
    "long": 2.07875338713492
  },
  ...
]
```

`/units`
```json
{
  "temperature": "°C",
  "wind_speed": "km/h",
  "gusts_speed": "km/h",
  "lat": "°",
  "wind_direction": "°",
  "last_update": "ISO 8601",
  "long": "°",
  "elevation": "m"
}
```

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
