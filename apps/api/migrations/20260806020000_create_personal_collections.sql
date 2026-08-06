CREATE TABLE user_collections (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    set_id UUID NOT NULL REFERENCES sets(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_collections_user_set_unique UNIQUE (user_id, set_id)
);

CREATE TABLE user_card_holdings (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    card_id UUID NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    quantity INTEGER NOT NULL,
    first_obtained_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_card_holdings_user_card_unique UNIQUE (user_id, card_id),
    CONSTRAINT user_card_holdings_quantity_positive CHECK (quantity > 0)
);

CREATE INDEX user_collections_user_created_idx
    ON user_collections (user_id, created_at DESC);
CREATE INDEX user_card_holdings_user_card_idx
    ON user_card_holdings (user_id, card_id);
