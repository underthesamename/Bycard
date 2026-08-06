CREATE TABLE games (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT games_slug_unique UNIQUE (slug),
    CONSTRAINT games_slug_format CHECK (slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    CONSTRAINT games_name_length CHECK (char_length(name) BETWEEN 1 AND 120)
);

CREATE TABLE sets (
    id UUID PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games(id) ON DELETE RESTRICT,
    external_key TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    series_name TEXT,
    release_date DATE NOT NULL,
    total_cards INTEGER NOT NULL,
    cover_image_url TEXT,
    language TEXT NOT NULL,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sets_external_key_unique UNIQUE (game_id, external_key, language),
    CONSTRAINT sets_slug_unique UNIQUE (game_id, slug),
    CONSTRAINT sets_external_key_format CHECK (external_key ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    CONSTRAINT sets_slug_format CHECK (slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    CONSTRAINT sets_name_length CHECK (char_length(name) BETWEEN 1 AND 160),
    CONSTRAINT sets_series_name_length CHECK (series_name IS NULL OR char_length(series_name) BETWEEN 1 AND 160),
    CONSTRAINT sets_total_cards_nonnegative CHECK (total_cards >= 0),
    CONSTRAINT sets_language_format CHECK (language ~ '^[a-z]{2}-[A-Z]{2}$')
);

CREATE TABLE cards (
    id UUID PRIMARY KEY,
    set_id UUID NOT NULL REFERENCES sets(id) ON DELETE CASCADE,
    external_key TEXT NOT NULL,
    local_number TEXT NOT NULL,
    printed_number TEXT NOT NULL,
    name TEXT NOT NULL,
    rarity TEXT,
    artist TEXT,
    image_small_url TEXT,
    image_large_url TEXT,
    sort_order INTEGER NOT NULL,
    metadata_json JSONB NOT NULL DEFAULT '{}'::JSONB,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cards_external_key_unique UNIQUE (set_id, external_key),
    CONSTRAINT cards_sort_order_unique UNIQUE (set_id, sort_order),
    CONSTRAINT cards_local_number_unique UNIQUE (set_id, local_number),
    CONSTRAINT cards_external_key_format CHECK (external_key ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    CONSTRAINT cards_name_length CHECK (char_length(name) BETWEEN 1 AND 160),
    CONSTRAINT cards_sort_order_nonnegative CHECK (sort_order >= 0),
    CONSTRAINT cards_metadata_object CHECK (jsonb_typeof(metadata_json) = 'object')
);

CREATE INDEX sets_release_date_idx ON sets (release_date DESC);
CREATE INDEX cards_set_name_idx ON cards (set_id, name);
