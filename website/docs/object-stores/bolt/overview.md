---
id: overview
title: Overview
sidebar_position: 1
---

# Bolt <small>object store</small>

[Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development object store because of these properties.

```hcl
object_store {
  engine = "bolt"
  boltdb {
    path = "/tmp/gofer-os.db"
  }
}
```

## Configuration

BoltDB needs to create a file on the local machine making the only parameter it accepts a path to the database file.

| Parameter | Type   | Default          | Description                         |
| --------- | ------ | ---------------- | ----------------------------------- |
| path      | string | /tmp/gofer-os.db | The path on disk to the boltdb file |
