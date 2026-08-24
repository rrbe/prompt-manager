CREATE TABLE prompt_id_sequence (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

INSERT INTO prompt_id_sequence(id)
SELECT MAX(id)
FROM prompts
HAVING COUNT(*) > 0;

DELETE FROM prompt_id_sequence;
