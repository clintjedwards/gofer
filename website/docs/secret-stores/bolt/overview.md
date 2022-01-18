---
id: overview
title: Overview
sidebar_position: 1
---

# Bolt <small>secret store</small>

[Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development object store because of these properties.

```hcl
object_store {
  engine = "bolt"
  boltdb {
    path           = "/tmp/gofer-os.db"
    encryption_key = "changemechangemechangemechangeme"
  }
}
```

## Configuration

BoltDB needs to create a file on the local machine making the first parameter it accepts a path to the database file. The second parameter should be a user created randomized string os that Bolt can encrypt secrets given to it at rest.

| Parameter      | Type   | Default                          | Description                                                  |
| -------------- | ------ | -------------------------------- | ------------------------------------------------------------ |
| path           | string | /tmp/gofer-os.db                 | The path on disk to the boltdb file                          |
| encryption_key | string | changemechangemechangemechangeme | A random 32 character string used to encrypt secrets at rest |
