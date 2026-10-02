# Filesystem <small>object store</small>

The filesystem object store keeps objects as files in a directory on the local machine. It is great for development and
small deployments.

```toml
[object_store]
engine = "filesystem"
pipeline_object_limit = 50
run_object_expiry = 2

[object_store.filesystem]
path = "/tmp/gofer_objects"
```

## Configuration

The filesystem store needs a directory on the local machine, making the only parameter it accepts a path to that
directory.

| Parameter | Type   | Default            | Description                                       |
| --------- | ------ | ------------------ | ------------------------------------------------- |
| path      | string | /tmp/gofer_objects | The path of the directory that holds object files |
