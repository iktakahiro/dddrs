CREATE TABLE todos (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK (status IN ('not_started', 'in_progress', 'completed')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    completed_at INTEGER,
    version INTEGER NOT NULL DEFAULT 0,
    CHECK ((status = 'completed') = (completed_at IS NOT NULL))
);

CREATE INDEX todos_by_created_at ON todos (created_at);
