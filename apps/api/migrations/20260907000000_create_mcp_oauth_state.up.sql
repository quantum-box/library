CREATE TABLE mcp_oauth_clients (
    client_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin PRIMARY KEY,
    metadata JSON NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE mcp_oauth_codes (
    code_hash BINARY(32) PRIMARY KEY,
    payload BLOB NOT NULL,
    expires_at DATETIME(6) NOT NULL,
    token_expires_at DATETIME(6) NOT NULL,
    INDEX mcp_oauth_codes_expiry (expires_at)
);
