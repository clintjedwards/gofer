# Sqlite <small>secret store</small>

The sqlite object store is great for development and small deployments.

```hcl
secret_store {
  engine = "sqlite"
  sqlite {
    path = "/tmp/gofer-secret.db"
    encryption_key = "changemechangemechangemechangeme"
  }
}
```

## Configuration

Sqlite needs to create a file on the local machine making the only parameter it accepts a path to the database file.

| Parameter      | Type   | Default              | Description                                  |
| -------------- | ------ | -------------------- | -------------------------------------------- |
| path           | string | /tmp/gofer-secret.db | The path on disk to the sqlite b file        |
| encryption_key | string | <required>           | 32 character key required to encrypt secrets |
