# API Reference

## Storage API (since 0.1.0)

The `Storage` struct provides methods for interacting with the database. This section documents the methods relevant to recommendations and car setups, added in PR #84.

### Recommendations

#### Insert Recommendation

```rust
fn insert_recommendation(
    &self,
    session_id: i64,
    lap_id: Option<i64>,
    category: &str,
    parameter: Option<&str>,
    confidence: &str,
    payload_json: &Value,
) -> Result<i64, StorageError>
```

- **Purpose:** Insert a new recommendation row into the `recommendations` table.
- **Returns:** The `rowid` of the inserted row on success.
- **Errors:** Returns `StorageError::Schema` if `category` or `confidence` is invalid.
- **Valid categories:**
  - `tires`, `gearing`, `alignment`, `anti_roll`, `springs`, `damping`, `aero`, `brakes`, `differential`, `ride_height`, `engine`
- **Valid confidences:**
  - `high`, `medium`, `low`

#### List Recommendations for Session

```rust
fn list_recommendations_for_session(
    &self,
    session_id: i64,
) -> Result<Vec<RecommendationRow>, StorageError>
```

- **Purpose:** Return all recommendations for a session, ordered by `created_at ASC`.
- **Returns:** Vector of `RecommendationRow`.

#### List Recommendations for Lap

```rust
fn list_recommendations_for_lap(
    &self,
    lap_id: i64,
) -> Result<Vec<RecommendationRow>, StorageError>
```

- **Purpose:** Return all recommendations associated with a specific lap, ordered by `created_at ASC`.
- **Returns:** Vector of `RecommendationRow`.

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
    pub payload_json: Value,
    pub schema_version: i32,
}
```

### Car Setups

#### Read Car Setup

```rust
fn read_car_setup(
    &self,
    car_ordinal: i32,
) -> Result<Option<CarSetup>, StorageError>
```

- **Purpose:** Look up the current setup for a car by ordinal.
- **Returns:** `Ok(None)` if no row exists for `car_ordinal`.
- **CarSetup Structure:**

```rust
pub struct CarSetup {
    pub setup: serde_json::Map<String, Value>,
    pub locked_params: Vec<String>,
    pub upgrades: serde_json::Map<String, Value>,
    pub source: String,
    pub schema_version: i32,
    pub updated_at: String,
}
```

### StorageError Variants

- `Sqlite(rusqlite::Error)`
- `Json(serde_json::Error)`
- `Schema(String)` — schema constraint violated

---

For more details on the storage schema, see [ADR 0001](../adr/0001-storage-schema.md).
