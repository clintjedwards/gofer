---
id: overview
title: Overview
sidebar_position: 1
---

# Bolt <small>storage</small>

[Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development database because of these properties.

```hcl
database {
  engine            = "bolt"
  max_results_limit = 100
  boltdb {
    path = "/tmp/gofer.db"
  }
}
```

## Configuration

BoltDB needs to create a file on the local machine making the only parameter it accepts a path to the database file.

| Parameter | Type   | Default       | Description                         |
| --------- | ------ | ------------- | ----------------------------------- |
| path      | string | /tmp/gofer.db | The path on disk to the boltdb file |
