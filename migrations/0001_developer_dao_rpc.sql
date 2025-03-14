CREATE TYPE PLAN AS ENUM('free', 'tier1', 'tier2', 'tier3');
CREATE TYPE CHAIN AS ENUM('optimism', 'polygon', 'arbitrum', 'base', 'anvil');
CREATE TYPE ASSET AS ENUM('ether', 'usdc');
CREATE TYPE ROLE AS ENUM('normie', 'admin');

CREATE TABLE IF NOT EXISTS Customers (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    wallet VARCHAR(42) UNIQUE,
    password VARCHAR(120) NOT NULL,
    role ROLE NOT NULL,
    plan PLAN NOT NULL,
    planExpiration TIMESTAMPTZ,
    verificationCode VARCHAR(10) NOT NULL,
    calls BIGINT NOT NULL,
    -- after the month is up, remaining calls get added here
    -- PAID PLANS ONLY
    leftoverCalls BIGINT NOT NULL,
    balance BIGINT NOT NULL,
    activated bool NOT NULL
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
    decimals INT NOT NULL,
    chain CHAIN NOT NULL,
    date TIMESTAMPTZ NOT NULL,
    usdValue BIGINT NOT NULL
);

-- SELECT asset, amount, chain, date, transactionHash FROM Payments where customerEmail = $1 AND data > $2 

-- registration (create account) -> select plan -> payments --tx_hash-> hits server -> confirm everything -> database + credit account 

CREATE INDEX IF NOT EXISTS idx_customer_email ON Payments (customerEmail);

-- pay as you go ** 
-- min deposit ? 
-- top up notifications 
