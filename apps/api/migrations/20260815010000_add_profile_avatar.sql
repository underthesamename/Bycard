ALTER TABLE users
    ADD COLUMN avatar_data BYTEA,
    ADD COLUMN avatar_content_type TEXT,
    ADD COLUMN avatar_version UUID,
    ADD CONSTRAINT users_avatar_fields_consistent CHECK (
        (avatar_data IS NULL AND avatar_content_type IS NULL AND avatar_version IS NULL)
        OR
        (avatar_data IS NOT NULL AND avatar_content_type = 'image/jpeg' AND avatar_version IS NOT NULL)
    ),
    ADD CONSTRAINT users_avatar_size CHECK (
        avatar_data IS NULL OR octet_length(avatar_data) <= 131072
    );
