use crate::storage::{map_rusqlite_error, StorageError};
use futures::TryFutureExt;
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct Role {
    pub id: String,
    pub description: String,
    pub permissions: String,
    pub system_role: bool,
}

impl From<&Row<'_>> for Role {
    fn from(row: &Row) -> Self {
        Self {
            id: row.get_unwrap("id"),
            description: row.get_unwrap("description"),
            permissions: row.get_unwrap("permissions"),
            system_role: row.get_unwrap("system_role"),
        }
    }
}

#[derive(Iden)]
enum RoleTable {
    Table,
    Id,
    Description,
    Permissions,
    SystemRole,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub description: Option<String>,
    pub permissions: Option<String>,
}

pub fn insert(conn: &mut Connection, role: &Role) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(RoleTable::Table)
        .columns([
            RoleTable::Id,
            RoleTable::Description,
            RoleTable::Permissions,
            RoleTable::SystemRole,
        ])
        .values_panic([
            role.id.clone().into(),
            role.description.clone().into(),
            role.permissions.clone().into(),
            role.system_role.into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &mut Connection) -> Result<Vec<Role>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            RoleTable::Id,
            RoleTable::Description,
            RoleTable::Permissions,
            RoleTable::SystemRole,
        ])
        .from(RoleTable::Table)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut roles: Vec<Role> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        roles.push(Role::from(row));
    }

    Ok(roles)
}

pub fn get(conn: &mut Connection, id: &str) -> Result<Role, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            RoleTable::Id,
            RoleTable::Description,
            RoleTable::Permissions,
            RoleTable::SystemRole,
        ])
        .from(RoleTable::Table)
        .and_where(Expr::col(RoleTable::Id).eq(id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Role::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &mut Connection,
    id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query
        .table(RoleTable::Table)
        .and_where(Expr::col(RoleTable::Id).eq(id));

    if let Some(value) = fields.permissions {
        query.value(RoleTable::Permissions, value.into());
    }

    if let Some(value) = fields.description {
        query.value(RoleTable::Description, value.into());
    }

    // If no fields were updated, return an error
    if query.values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(conn: &mut Connection, id: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(RoleTable::Table)
        .and_where(Expr::col(RoleTable::Id).eq(id))
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

        let role = Role {
            id: "some_id".into(),
            description: "some_description".into(),
            permissions: "permissioning".into(),
            system_role: false,
        };

        insert(&mut conn, &role)?;

        Ok((harness, conn))
    }

    fn test_list_roles() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let roles = list(&mut conn).expect("Failed to list roles");

        // Assert that we got at least one role back
        assert!(!roles.is_empty(), "No roles returned");

        for role in roles {
            match role.id.as_str() {
                "some_id" => {
                    assert_eq!(role.permissions, "permissioning");
                }
                _ => panic!("Unexpected role"),
            }
        }
    }

    fn test_update_roles() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            description: None,
            permissions: Some("some_permissioning".into()),
        };

        update(&mut conn, "some_id", fields_to_update.clone()).expect("Failed to update role");

        let updated_role = get(&mut conn, "some_id").expect("Failed to retrieve updated role");

        assert_eq!(
            fields_to_update.permissions.unwrap(),
            updated_role.permissions
        );
    }

    fn test_insert_and_get() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fetched_role = get(&mut conn, "some_id").expect("Failed to get Role");
        assert_eq!(fetched_role.id, "some_id");
        assert_eq!(fetched_role.permissions, "permissioning",);
    }

    fn test_delete() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id").expect("Failed to delete Role");

        let result = get(&mut conn, "some_id");
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
