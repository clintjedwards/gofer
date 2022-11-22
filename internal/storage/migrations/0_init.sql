CREATE TABLE IF NOT EXISTS namespaces (
    id          TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    created     INTEGER NOT NULL,
    modified    INTEGER NOT NULL,
    PRIMARY KEY (id)
) STRICT;

CREATE TABLE IF NOT EXISTS pipelines (
    namespace   TEXT    NOT NULL,
    id          TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    parallelism INTEGER NOT NULL,
    created     INTEGER NOT NULL,
    modified    INTEGER NOT NULL,
    state       TEXT    NOT NULL,
    errors      TEXT    NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, id)
) STRICT;

CREATE INDEX idx_created_pipelines ON pipelines (created);

CREATE TABLE IF NOT EXISTS pipeline_trigger_settings (
    namespace TEXT NOT NULL,
    pipeline  TEXT NOT NULL,
    name      TEXT NOT NULL,
    label     TEXT NOT NULL,
    settings  TEXT,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, label)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_common_task_settings (
    namespace        TEXT NOT NULL,
    pipeline         TEXT NOT NULL,
    name             TEXT NOT NULL,
    label            TEXT NOT NULL,
    settings         TEXT,
    inject_api_token INTEGER CHECK (inject_api_token IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, label)
) STRICT;

CREATE TABLE IF NOT EXISTS runs (
    namespace    TEXT    NOT NULL,
    pipeline     TEXT    NOT NULL,
    id           INTEGER NOT NULL,
    started      INTEGER NOT NULL,
    ended        INTEGER NOT NULL,
    state        TEXT    NOT NULL,
    status       TEXT    NOT NULL,
    status_reason TEXT,
    trigger      TEXT    NOT NULL,
    variables    TEXT    NOT NULL,
    store_objects_expired  INTEGER NOT NULL CHECK (store_objects_expired IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE INDEX idx_runs_started ON runs (started);

CREATE TABLE IF NOT EXISTS custom_tasks (
    namespace        TEXT NOT NULL,
    pipeline         TEXT NOT NULL,
    id               TEXT NOT NULL,
    description      TEXT,
    image            TEXT NOT NULL,
    registry_auth    TEXT,
    depends_on       TEXT NOT NULL,
    variables        TEXT NOT NULL,
    entrypoint       TEXT,
    command          TEXT,
    inject_api_token INTEGER CHECK (inject_api_token IN (0, 1)),
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE TABLE IF NOT EXISTS trigger_registrations (
    name          TEXT    NOT NULL,
    image         TEXT    NOT NULL,
    registry_auth TEXT,
    variables     TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    status        TEXT    NOT NULL,
    PRIMARY KEY (name)
) STRICT;

CREATE TABLE IF NOT EXISTS common_task_registrations (
    name          TEXT    NOT NULL,
    image         TEXT    NOT NULL,
    registry_auth TEXT,
    variables     TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    status        TEXT    NOT NULL,
    documentation TEXT    NOT NULL,
    PRIMARY KEY (name)
) STRICT;

CREATE TABLE IF NOT EXISTS object_store_pipeline_keys(
    namespace     TEXT NOT NULL,
    pipeline      TEXT NOT NULL,
    key           TEXT NOT NULL,
    created       INTEGER NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, key)
) STRICT;

CREATE INDEX idx_object_pipeline_keys_created ON object_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS object_store_run_keys(
    namespace     TEXT NOT NULL,
    pipeline      TEXT NOT NULL,
    run           INTEGER NOT NULL,
    key           TEXT NOT NULL,
    created       INTEGER NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline, run) REFERENCES runs(namespace, pipeline, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, run, key)
) STRICT;

CREATE INDEX idx_object_run_keys_created ON object_store_run_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_pipeline_keys(
    namespace     TEXT NOT NULL,
    pipeline      TEXT NOT NULL,
    key           TEXT NOT NULL,
    created       INTEGER NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, key)
) STRICT;

CREATE INDEX idx_secret_pipeline_keys_created ON secret_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_global_keys(
    key           TEXT NOT NULL,
    created       INTEGER NOT NULL,
    PRIMARY KEY (key)
) STRICT;

CREATE INDEX idx_secret_global_keys_created ON secret_store_global_keys (created);

CREATE TABLE IF NOT EXISTS task_runs (
    namespace     TEXT    NOT NULL,
    pipeline      TEXT    NOT NULL,
    run           INTEGER NOT NULL,
    id            TEXT    NOT NULL,
    task          TEXT    NOT NULL,
    created       INTEGER NOT NULL,
    started       INTEGER NOT NULL,
    ended         INTEGER NOT NULL,
    exit_code     INTEGER,
    logs_expired  INTEGER NOT NULL CHECK (logs_expired IN (0, 1)),
    logs_removed  INTEGER NOT NULL CHECK (logs_removed IN (0, 1)),
    state         TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    status_reason TEXT,
    variables     TEXT NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, run, id)
) STRICT;

CREATE INDEX idx_taskruns_started ON task_runs (started);

CREATE TABLE IF NOT EXISTS events (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    kind     TEXT NOT NULL,
    details  TEXT NOT NULL,
    emitted  INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS tokens (
    hash        TEXT NOT NULL,
    created     INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    namespaces  TEXT NOT NULL,
    metadata    TEXT,
    expires     TEXT NOT NULL,
    disabled    INTEGER NOT NULL CHECK (disabled IN (0, 1)),
    PRIMARY KEY (hash)
) STRICT;
