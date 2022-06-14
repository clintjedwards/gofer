PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
PRAGMA foreign_keys = ON;
PRAGMA strict = ON;

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
    store_keys  TEXT    NOT NULL,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, id)
) STRICT;

CREATE INDEX idx_created_pipelines ON pipelines (created);

CREATE TABLE IF NOT EXISTS pipeline_trigger_settings (
    namespace TEXT NOT NULL,
    pipeline  TEXT NOT NULL,
    kind      TEXT NOT NULL,
    label     TEXT NOT NULL,
    settings  TEXT,
    error     TEXT,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, label)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_notifier_settings (
    namespace TEXT NOT NULL,
    pipeline  TEXT NOT NULL,
    kind      TEXT NOT NULL,
    label     TEXT NOT NULL,
    settings  TEXT,
    error     TEXT,
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
    failure_info TEXT,
    task_runs    TEXT    NOT NULL,
    trigger      TEXT    NOT NULL,
    variables    TEXT    NOT NULL,
    store_info   TEXT,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE INDEX idx_runs_started ON runs (started);

CREATE TABLE IF NOT EXISTS tasks (
    namespace     TEXT NOT NULL,
    pipeline      TEXT NOT NULL,
    id            TEXT NOT NULL,
    description   TEXT,
    image         TEXT NOT NULL,
    registry_auth TEXT,
    depends_on    TEXT NOT NULL,
    variables     TEXT NOT NULL,
    exec          TEXT,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, id)
) STRICT;

CREATE TABLE IF NOT EXISTS triggers (
    id TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS notifiers (
    id TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS object_store_run_keys(
    id TEXT NOT NULL
) STRICT;

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
    failure       TEXT,
    logs_expired  INTEGER NOT NULL CHECK (logs_expired IN (0, 1)),
    logs_removed  INTEGER NOT NULL CHECK (logs_removed IN (0, 1)),
    state         TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    scheduler_id  TEXT,
    FOREIGN KEY (namespace) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace, pipeline) REFERENCES pipelines(namespace, id) ON DELETE CASCADE,
    PRIMARY KEY (namespace, pipeline, run, id)
) STRICT;
