ALTER TABLE users ADD COLUMN username TEXT;

UPDATE users
SET username = 'colecionador-' || substr(replace(id::text, '-', ''), 1, 8)
WHERE username IS NULL;

ALTER TABLE users ALTER COLUMN username SET NOT NULL;
ALTER TABLE users
    ADD CONSTRAINT users_username_length CHECK (char_length(username) BETWEEN 3 AND 24),
    ADD CONSTRAINT users_username_format CHECK (username ~ '^[a-z0-9]+(?:[._-][a-z0-9]+)*$'),
    ADD CONSTRAINT users_username_normalized CHECK (username = lower(username) AND username = btrim(username)),
    ADD CONSTRAINT users_username_unique UNIQUE (username);
