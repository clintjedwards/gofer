# Sqlite <small>object store</small>

The sqlite object store is great for development and small deployments.

```hcl
object_store {
  engine = "sqlite"
  sqlite {
    path = "/tmp/gofer-object.db"
  }
}
```

## Configuration

Sqlite needs to create a file on the local machine making the only parameter it accepts a path to the database file.

| Parameter | Type   | Default              | Description                            |
| --------- | ------ | -------------------- | -------------------------------------- |
| path      | string | /tmp/gofer-object.db | The path on disk to the sqlite db file |
