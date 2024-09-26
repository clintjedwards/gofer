use crate::storage::{map_rusqlite_error, Executable, StorageError};
use rusqlite::Row;
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub details: String,
    pub emitted: String,
}

impl From<&Row<'_>> for Event {
    fn from(row: &Row) -> Self {
        Self {
            id: row.get_unwrap("id"),
            kind: row.get_unwrap("kind"),
            details: row.get_unwrap("details"),
            emitted: row.get_unwrap("emitted"),
        }
    }
}

#[derive(Iden)]
enum EventTable {
    Table,
    Id,
    Kind,
    Details,
    Emitted,
}

pub fn insert(conn: &dyn Executable, event: &Event) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(EventTable::Table)
        .columns([
            EventTable::Id,
            EventTable::Kind,
            EventTable::Details,
            EventTable::Emitted,
        ])
        .values_panic([
            event.id.clone().into(),
            event.kind.clone().into(),
            event.details.clone().into(),
            event.emitted.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &dyn Executable,
    offset: i64,
    limit: i64,
    reverse: bool,
) -> Result<Vec<Event>, StorageError> {
    let order_direction = if reverse { Order::Desc } else { Order::Asc };

    let (sql, values) = Query::select()
        .columns([
            EventTable::Id,
            EventTable::Kind,
            EventTable::Details,
            EventTable::Emitted,
        ])
        .from(EventTable::Table)
        .order_by(EventTable::Id, order_direction)
        .limit(limit as u64)
        .offset(offset as u64)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Event> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Event::from(row));
    }

    Ok(objects)
}

pub fn get(conn: &dyn Executable, id: &str) -> Result<Event, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            EventTable::Id,
            EventTable::Kind,
            EventTable::Details,
            EventTable::Emitted,
        ])
        .from(EventTable::Table)
        .and_where(Expr::col(EventTable::Id).eq(id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Event::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn delete(conn: &dyn Executable, id: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(EventTable::Table)
        .and_where(Expr::col(EventTable::Id).eq(id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{tests::TestHarness, Executable};

    fn setup() -> Result<(TestHarness, impl Executable), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

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

        insert(&mut conn, &event)?;
        insert(&mut conn, &event2)?;

        Ok((harness, conn))
    }

    fn test_list_events() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let events = list(&mut conn, 0, 10, false).expect("Failed to list events");

        assert_eq!(events.len(), 2);

        let events =
            list(&mut conn, 1, 1, false).expect("Failed to list events with limit and offset");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].id, "some_id_2",
            "The event id should match the second inserted event."
        );

        let events_reverse = list(&mut conn, 0, 2, false).expect("Failed to list events in order");

        assert_eq!(events_reverse.len(), 2, "Should fetch two events.");
        assert_eq!(
            events_reverse[0].id, "some_id",
            "The first event should be the first one inserted."
        );
        assert_eq!(
            events_reverse[1].id, "some_id_2",
            "The second event should be the second one inserted."
        );

        let events_reverse =
            list(&mut conn, 0, 2, true).expect("Failed to list events in reverse order");

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

    fn test_get_event() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let event = get(&mut conn, "some_id").expect("Failed to get event");

        assert_eq!(event.id, "some_id");
        assert_eq!(event.kind, "some_kind");
        assert_eq!(event.details, "some_details");
        assert_eq!(event.emitted, "some_time");

        assert!(
            get(&mut conn, "non_existent_id").is_err(),
            "Should not have found an event"
        );
    }

    fn test_delete_event() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id").expect("Failed to delete event");

        assert!(
            get(&mut conn, "some_id").is_err(),
            "Event should have been deleted"
        );
    }
}
