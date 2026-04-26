# API Reference

## Storage API (Rust)

### Recommendations

The storage layer provides methods to insert and query recommendations associated with sessions and laps.

#### Valid Recommendation Categories

The following values are valid for the `recommendations.category` column:

- `tires`
- `gearing`
- `alignment`
- `anti_roll`
- `springs`
- `damping`
- `aero`
- `brakes`
- `differential`

#### RecommendationRow Structure

```rust
pub struct RecommendationRow {
    pub id: i64,
    pub session_id: i64,
    pub lap_id: Option<i64>,
    pub created_at: String,
    pub category: String,
    pub parameter: Option<String>,
    pub confidence: String,
    pub delivered: bool,
    pub dismissed: bool,
    pub payload_json: serde_json::Value,
    pub schema_version: i32,
}
```

#### Methods

- **insert_recommendation**
  - Inserts a new recommendation row.
  - Returns the rowid of the inserted row.
  - Errors with `StorageError::Schema` if `category` is not valid.
  - Arguments:
    - `session_id: i64`
    - `lap_id: Option<i64>`
    - `category: &str`
    - `parameter: Option<&str>`
    - `confidence: &str`
    - `payload_json: &Value`

- **list_recommendations_for_session**
  - Returns all recommendations for a session, ordered by `created_at ASC`.
  - Arguments:
    - `session_id: i64`
  - Returns: `Vec<RecommendationRow>`

- **list_recommendations_for_lap**
  - Returns all recommendations for a specific lap, ordered by `created_at ASC`.
  - Arguments:
    - `lap_id: i64`
  - Returns: `Vec<RecommendationRow>`

### Car Setups

#### CarSetup Structure

```rust
pub struct CarSetup {
    pub setup: serde_json::Map<String, Value>,
    pub locked_params: Vec<String>,
    pub upgrades: serde_json::Map<String, Value>,
    pub source: String,
}
```

#### Method

- **read_car_setup**
  - Looks up the current setup for a car by ordinal.
  - Returns `Ok(None)` if no row exists for the given `car_ordinal`.
  - Arguments:
    - `car_ordinal: i32`
  - Returns: `Option<CarSetup>`

### StorageError

New variant:
- `Schema(String)` — schema constraint violated (e.g., invalid recommendation category).

---

_Last updated for sidecar storage.rs changes in PR #84._
