use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub details: String,
    pub emitted: String,
}

pub async fn insert(conn: &mut SqliteConnection, event: &Event) -> Result<(), StorageError> {
    let query = sqlx::query("INSERT INTO events (id, kind, details, emitted) VALUES (?, ?, ?, ?);")
        .bind(&event.id)
        .bind(&event.kind)
        .bind(&event.details)
        .bind(&event.emitted);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(
    conn: &mut SqliteConnection,
    offset: i64,
    limit: i64,
    reverse: bool,
) -> Result<Vec<Event>, StorageError> {
    let order_direction = if reverse { "DESC" } else { "ASC" };

    let query = format!(
        "SELECT id, kind, details, emitted FROM events ORDER BY id {} LIMIT ? OFFSET ?;",
        order_direction
    );

    let query = sqlx::query_as::<_, Event>(&query).bind(limit).bind(offset);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn list_by_id(
    conn: &mut SqliteConnection,
    id: &str,
    limit: i64,
) -> Result<Vec<Event>, StorageError> {
    let query = sqlx::query_as::<_, Event>(
        "SELECT id, kind, details, emitted FROM events WHERE id >= ? ORDER BY id LIMIT ?;",
    )
    .bind(id)
    .bind(limit);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(conn: &mut SqliteConnection, id: &str) -> Result<Event, StorageError> {
    let query =
        sqlx::query_as::<_, Event>("SELECT id, kind, details, emitted FROM events WHERE id = ?;")
            .bind(id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete(conn: &mut SqliteConnection, id: &str) -> Result<(), StorageError> {
    let query = sqlx::query("DELETE FROM events WHERE id = ?;").bind(id);

    let sql = query.sql();

    query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        let event = Event {
            id: "some_id".into(),
            kind: "some_kind".into(),
            details: "some_details".into(),
            emitted: "some_time".into(),
        };

        let event2 = Event {
            id: "some_id_2".into(),
            kind: "some_kind".into(),
            details: "some_details".into(),
            emitted: "some_time".into(),
        };

        insert(&mut conn, &event).await?;
        insert(&mut conn, &event2).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_events() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let events = list(&mut conn, 0, 10, false)
            .await
            .expect("Failed to list events");

        assert_eq!(events.len(), 2);

        let events = list(&mut conn, 1, 1, false)
            .await
            .expect("Failed to list events with limit and offset");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].id, "some_id_2",
            "The event id should match the second inserted event."
        );

        let events_reverse = list(&mut conn, 0, 2, false)
            .await
            .expect("Failed to list events in order");

        assert_eq!(events_reverse.len(), 2, "Should fetch two events.");
        assert_eq!(
            events_reverse[0].id, "some_id",
            "The first event should be the first one inserted."
        );
        assert_eq!(
            events_reverse[1].id, "some_id_2",
            "The second event should be the second one inserted."
        );

        let events_reverse = list(&mut conn, 0, 2, true)
            .await
            .expect("Failed to list events in reverse order");

        assert_eq!(events_reverse.len(), 2, "Should fetch two events.");
        assert_eq!(
            events_reverse[0].id, "some_id_2",
            "The first event in reverse order should be the last one inserted."
        );
        assert_eq!(
            events_reverse[1].id, "some_id",
            "The second event in reverse order should be the second one inserted."
        );
    }

    #[tokio::test]
    async fn test_get_event() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let event = get(&mut conn, "some_id")
            .await
            .expect("Failed to get event");

        assert_eq!(event.id, "some_id");
        assert_eq!(event.kind, "some_kind");
        assert_eq!(event.details, "some_details");
        assert_eq!(event.emitted, "some_time");

        assert!(
            get(&mut conn, "non_existent_id").await.is_err(),
            "Should not have found an event"
        );
    }

    #[tokio::test]
    async fn test_delete_event() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete event");

        assert!(
            get(&mut conn, "some_id").await.is_err(),
            "Event should have been deleted"
        );
    }
}
