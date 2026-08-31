ALTER TABLE prompts
ADD COLUMN exec TEXT;

ALTER TABLE prompt_versions
ADD COLUMN exec TEXT;
