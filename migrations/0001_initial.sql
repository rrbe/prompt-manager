CREATE TABLE prompts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_used_at INTEGER,
    use_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE prompt_tags (
    prompt_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (prompt_id, tag_id),
    FOREIGN KEY (prompt_id) REFERENCES prompts(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE VIRTUAL TABLE prompts_fts USING fts5(
    name,
    description,
    content,
    content = 'prompts',
    content_rowid = 'id'
);

CREATE TRIGGER prompts_fts_insert AFTER INSERT ON prompts BEGIN
    INSERT INTO prompts_fts(rowid, name, description, content)
    VALUES (new.id, new.name, new.description, new.content);
END;

CREATE TRIGGER prompts_fts_delete AFTER DELETE ON prompts BEGIN
    INSERT INTO prompts_fts(prompts_fts, rowid, name, description, content)
    VALUES ('delete', old.id, old.name, old.description, old.content);
END;

CREATE TRIGGER prompts_fts_update AFTER UPDATE OF name, description, content ON prompts BEGIN
    INSERT INTO prompts_fts(prompts_fts, rowid, name, description, content)
    VALUES ('delete', old.id, old.name, old.description, old.content);
    INSERT INTO prompts_fts(rowid, name, description, content)
    VALUES (new.id, new.name, new.description, new.content);
END;
