# Sqlite <small>object store</small>

The sqlite object store is great for development and small deployments.

```toml
[object_store]
engine = "sqlite"
pipeline_object_limit = 50
run_object_expiry = 50

[object_store.sqlite]
path = "/tmp/gofer_objects.db"
```

## Configuration

Sqlite needs to create a file on the local machine making the only parameter it accepts a path to the database file.

| Parameter | Type   | Default              | Description                            |
| --------- | ------ | -------------------- | -------------------------------------- |
| path      | string | /tmp/gofer-object.db | The path on disk to the sqlite db file |
