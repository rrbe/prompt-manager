ALTER TABLE prompts
ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1));

CREATE TABLE prompt_versions (
    id INTEGER PRIMARY KEY,
    prompt_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (prompt_id, version),
    FOREIGN KEY (prompt_id) REFERENCES prompts(id) ON DELETE CASCADE
);

CREATE TABLE prompt_version_tags (
    prompt_version_id INTEGER NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (prompt_version_id, tag),
    FOREIGN KEY (prompt_version_id) REFERENCES prompt_versions(id) ON DELETE CASCADE
);

CREATE INDEX prompt_versions_prompt_id_version
ON prompt_versions(prompt_id, version DESC);

INSERT INTO prompt_versions(prompt_id, version, name, description, content, created_at)
SELECT id, 1, name, description, content, updated_at
FROM prompts;

INSERT INTO prompt_version_tags(prompt_version_id, tag)
SELECT prompt_versions.id, tags.name
FROM prompt_versions
JOIN prompt_tags ON prompt_tags.prompt_id = prompt_versions.prompt_id
JOIN tags ON tags.id = prompt_tags.tag_id;
