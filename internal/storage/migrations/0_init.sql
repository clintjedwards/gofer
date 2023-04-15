CREATE TABLE IF NOT EXISTS namespaces (
    id          TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    created     INTEGER NOT NULL,
    modified    INTEGER NOT NULL,
    PRIMARY KEY (id)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_metadata (
    namespace   TEXT    NOT NULL,
    id          TEXT    NOT NULL,
    created     INTEGER NOT NULL,
    modified    INTEGER NOT NULL,
    state       TEXT    NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, id)
) STRICT;

CREATE INDEX idx_created_pipeline_metadata ON pipeline_metadata (created);

CREATE TABLE IF NOT EXISTS pipeline_configs (
    namespace   TEXT    NOT NULL,
    pipeline    TEXT    NOT NULL,
    version     INTEGER NOT NULL,
    parallelism INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    registered  INTEGER NOT NULL,
    deprecated  INTEGER NOT NULL,
    state       TEXT    NOT NULL,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, version)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_deployments (
    namespace     TEXT    NOT NULL,
    pipeline      TEXT    NOT NULL,
    id            INTEGER NOT NULL,
    start_version INTEGER NOT NULL,
    end_version   INTEGER NOT NULL,
    started       INTEGER NOT NULL,
    ended         INTEGER NOT NULL,
    state         TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    status_reason TEXT    NOT NULL,
    logs          TEXT    NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_extension_subscriptions (
    namespace        TEXT NOT NULL,
    pipeline         TEXT NOT NULL,
    name             TEXT NOT NULL,
    label            TEXT NOT NULL,
    settings         TEXT NOT NULL,
    status           TEXT NOT NULL,
    status_reason    TEXT NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, name, label)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_common_task_settings (
    namespace               TEXT    NOT NULL,
    pipeline                TEXT    NOT NULL,
    pipeline_config_version INTEGER NOT NULL,
    name                    TEXT    NOT NULL,
    label                   TEXT    NOT NULL,
    description             TEXT    NOT NULL,
    depends_on              TEXT    NOT NULL,
    settings                TEXT    NOT NULL,
    inject_api_token        INTEGER NOT NULL CHECK (inject_api_token IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, pipeline_config_version) REFERENCES pipeline_configs(namespace, pipeline, version) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, pipeline_config_version, label)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_runs (
    namespace               TEXT    NOT NULL,
    pipeline                TEXT    NOT NULL,
    pipeline_config_version INTEGER NOT NULL,
    id                      INTEGER NOT NULL,
    started                 INTEGER NOT NULL,
    ended                   INTEGER NOT NULL,
    state                   TEXT    NOT NULL,
    status                  TEXT    NOT NULL,
    status_reason           TEXT    NOT NULL,
    initiator               TEXT    NOT NULL,
    variables               TEXT    NOT NULL,
    store_objects_expired   INTEGER NOT NULL CHECK (store_objects_expired IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, pipeline_config_version) REFERENCES pipeline_configs(namespace, pipeline, version),
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_custom_tasks (
    namespace               TEXT    NOT NULL,
    pipeline                TEXT    NOT NULL,
    pipeline_config_version INTEGER NOT NULL,
    id                      TEXT    NOT NULL,
    description             TEXT    NOT NULL,
    image                   TEXT    NOT NULL,
    registry_auth           TEXT    NOT NULL,
    depends_on              TEXT    NOT NULL,
    variables               TEXT    NOT NULL,
    entrypoint              TEXT    NOT NULL,
    command                 TEXT    NOT NULL,
    inject_api_token        INTEGER NOT NULL CHECK (inject_api_token IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, pipeline_config_version) REFERENCES pipeline_configs(namespace, pipeline, version) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, pipeline_config_version, id)
) STRICT;

CREATE TABLE IF NOT EXISTS global_extension_registrations (
    name          TEXT    NOT NULL,
    image         TEXT    NOT NULL,
    registry_auth TEXT    NOT NULL,
    variables     TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    status        TEXT    NOT NULL,
    key_id        INTEGER NOT NULL,
    PRIMARY KEY (name)
) STRICT;

CREATE TABLE IF NOT EXISTS global_common_task_registrations (
    name          TEXT    NOT NULL,
    image         TEXT    NOT NULL,
    registry_auth TEXT    NOT NULL,
    variables     TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    status        TEXT    NOT NULL,
    documentation TEXT    NOT NULL,
    PRIMARY KEY (name)
) STRICT;

CREATE TABLE IF NOT EXISTS object_store_pipeline_keys(
    namespace     TEXT     NOT NULL,
    pipeline      TEXT     NOT NULL,
    key           TEXT     NOT NULL,
    created       INTEGER  NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, key)
) STRICT;

CREATE INDEX idx_object_pipeline_keys_created ON object_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS object_store_run_keys(
    namespace     TEXT    NOT NULL,
    pipeline      TEXT    NOT NULL,
    run           INTEGER NOT NULL,
    key           TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, run) REFERENCES pipeline_runs(namespace, pipeline, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, run, key)
) STRICT;

CREATE INDEX idx_object_run_keys_created ON object_store_run_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_pipeline_keys(
    namespace     TEXT    NOT NULL,
    pipeline      TEXT    NOT NULL,
    key           TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, key)
) STRICT;

CREATE INDEX idx_secret_pipeline_keys_created ON secret_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_global_keys(
    key           TEXT    NOT NULL,
    namespaces    TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    PRIMARY KEY (key)
) STRICT;

CREATE INDEX idx_secret_global_keys_created ON secret_store_global_keys (created);

CREATE TABLE IF NOT EXISTS pipeline_task_runs (
    namespace     TEXT    NOT NULL,
    pipeline      TEXT    NOT NULL,
    run           INTEGER NOT NULL,
    id            TEXT    NOT NULL,
    task          TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    started       INTEGER NOT NULL,
    ended         INTEGER NOT NULL,
    exit_code     INTEGER NOT NULL,
    logs_expired  INTEGER NOT NULL CHECK (logs_expired IN (0, 1)),
    logs_removed  INTEGER NOT NULL CHECK (logs_removed IN (0, 1)),
    state         TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    status_reason TEXT    NOT NULL,
    variables     TEXT    NOT NULL,
    task_kind     TEXT    NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipeline_metadata(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, run) REFERENCES pipeline_runs(namespace, pipeline, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, run, id)
) STRICT;

CREATE INDEX idx_taskruns_started ON pipeline_task_runs (started);

CREATE TABLE IF NOT EXISTS events (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    type     TEXT    NOT NULL,
    details  TEXT    NOT NULL,
    emitted  INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS tokens (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    hash        TEXT    NOT NULL,
    created     INTEGER NOT NULL,
    kind        TEXT    NOT NULL,
    namespaces  TEXT    NOT NULL,
    metadata    TEXT    NOT NULL,
    expires     INTEGER NOT NULL,
    disabled    INTEGER NOT NULL CHECK (disabled IN (0, 1))
) STRICT;

CREATE INDEX idx_tokens_hash ON tokens (hash);

