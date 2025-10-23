CREATE TYPE PLAN AS ENUM('free', 'tier1', 'tier2', 'tier3');
CREATE TYPE CHAIN AS ENUM('optimism', 'polygon', 'arbitrum', 'base', 'anvil', 'sepolia');
CREATE TYPE ASSET AS ENUM('ether', 'usdc');
CREATE TYPE ROLE AS ENUM('normie', 'admin');

CREATE TABLE IF NOT EXISTS Customers (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    wallet VARCHAR(42) UNIQUE,
    password VARCHAR(120) NOT NULL,
    role ROLE NOT NULL default 'normie',
    verificationCode VARCHAR(10) NOT NULL,
    nonce TEXT NOT NULL,
    balance BIGINT CHECK (balance >= 0) NOT NULL default 0,
    created TIMESTAMPTZ GENERATED ALWAYS AS(('now'::timestamptz AT TIME ZONE 'UTC')) STORED NOT NULL,
    activated bool NOT NULL
);

CREATE TABLE IF NOT EXISTS RpcPlans (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    calls BIGINT CHECK (calls >= 0) NOT NULL default 0,
    plan PLAN NOT NULL DEFAULT 'free',
    created TIMESTAMPTZ GENERATED ALWAYS AS(('now'::timestamptz AT TIME ZONE 'UTC')) STORED NOT NULL,
    expires TIMESTAMPTZ GENERATED ALWAYS AS(('now'::timestamptz AT TIME ZONE 'UTC') + INTERVAL '1 months') STORED NOT NULL
);

CREATE TABLE IF NOT EXISTS Api (
    customerEmail VARCHAR(255),
    apiKey VARCHAR(255),
    PRIMARY KEY(customerEmail, apiKey)
);

CREATE TABLE IF NOT EXISTS Payments (
    customerEmail VARCHAR(255) NOT NULL, 
    transactionHash VARCHAR(120) PRIMARY KEY,  -- Unique for each payment
    asset ASSET NOT NULL, 
    amount TEXT NOT NULL,
    -- must know precision for storing the raw amounts as bigint
    decimals INT CHECK(decimals > 0) NOT NULL,
    chain CHAIN NOT NULL,
    date TIMESTAMPTZ GENERATED ALWAYS AS(('now'::timestamptz AT TIME ZONE 'UTC')) STORED NOT NULL,
    usdValue BIGINT CHECK(usdValue > 0) NOT NULL
);

-- SELECT asset, amount, chain, date, transactionHash FROM Payments where customerEmail = $1 AND data > $2 

-- registration (create account) -> select plan -> payments --tx_hash-> hits server -> confirm everything -> database + credit account 

CREATE INDEX IF NOT EXISTS idx_customer_email ON Payments (customerEmail);

-- pay as you go ** 
-- min deposit ? 
-- top up notifications

CREATE TABLE event_subscriptions (
    id UUID PRIMARY KEY,
    user_email VARCHAR(255) REFERENCES Customers(email),
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    event_signature TEXT, -- NULL = all events
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_email, chain_id, contract_address, event_signature)
);

CREATE TABLE tracked_events (
    id UUID PRIMARY KEY,
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    event_signature TEXT NOT NULL,
    event_data JSONB NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    confirmed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(chain_id, tx_hash, log_index)
);

-- Track sync progress per contract
CREATE TABLE event_sync_state (
  chain_id TEXT NOT NULL,
  contract_address TEXT NOT NULL,
  last_synced_block BIGINT NOT NULL,
  last_synced_at TIMESTAMPTZ DEFAULT NOW(),
  PRIMARY KEY(chain_id, contract_address)
);