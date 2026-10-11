# Sqlite <small>secret store</small>

The sqlite secret store is great for development and small deployments.

```toml
[secret_store]
engine = "sqlite"

[secret_store.sqlite]
path = "/tmp/gofer_secrets.db"
encryption_key = "changemechangemechangemechangeme"
```

## Configuration

Sqlite keeps secrets in a database file on the local machine, encrypted with the key you give it.

| Parameter      | Type   | Default                            | Description                                                                                                                                         |
| -------------- | ------ | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| path           | string | /tmp/gofer_secrets.db              | The path on disk to the sqlite database file.                                                                                                       |
| encryption_key | string | changemechangemechangemechangeme   | Key used to encrypt secrets. Must be exactly 32 bytes (32 ASCII characters); `openssl rand -hex 16` makes one. Always change the default in production. |
