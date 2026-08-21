# Daily Server

This is a small REST API for my roguelike game [Bleeding in the Blur](https://github.com/PaoloMazzon/BleedingInTheBlur)
so it may eventually have daily runs and leaderboards. It's a pretty simple highscore API wrapped around an SQLite
database written in Rust using Tokio + Axum.

## API
The public API for this server is very consistent, all incoming and outgoing data is JSON. Any non-200 status code will
use the same JSON format:

```json
{
  "error": "..."
}
```

### `POST` - `/api/v1/submit`
Request
```json
{
  "name": "...",
  "extra_data": "{}",
  "score": 123,
  "daily_seed": 123
}
```
Response:
```json
{
  "id": 123
}
```

### `GET` - `/api/v1/daily-seed`
No encoding in the URI needed. Response:
```json
{
  "seed": 123
}
```

### `GET` - `/api/v1/leaderboard`
The following request should be encoded in the URI
```json
{
  "starting_index": 0,
  "count": 10,
  "date": "2026-12-31"
}
```
Starting index is relative to the sorted leaderboard, not arbitrary indices.

Response:
```json
{
  "scores": [
    {
      "id": 123,
      "name": "...",
      "extra_data":  "{}",
      "score": 321,
      "date":  "2026-12-31"
    }
  ]
}
```