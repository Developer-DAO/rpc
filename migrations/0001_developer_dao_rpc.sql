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
    expires TIMESTAMPTZ GENERATED ALWAYS AS(('now'::timestamptz AT TIME ZONE 'UTC') + INTERVAL '1 months') STORED NOT NULL,
    downgradeTo PLAN
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

CREATE INDEX IF NOT EXISTS idx_email_payment ON Payments (customerEmail);
CREATE INDEX IF NOT EXISTS idx_email_rpc ON RpcPlans (email);

-- pay as you go ** 
-- min deposit ? 
-- top up notifications 
