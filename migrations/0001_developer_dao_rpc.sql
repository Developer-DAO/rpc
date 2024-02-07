CREATE TYPE PLAN AS ENUM('none', 'based', 'premier', 'gigachad');
CREATE TYPE CHAIN AS ENUM('optimism', 'polygon', 'arbitrum', 'base');
CREATE TYPE ASSET AS ENUM('ether', 'usdc');
CREATE TYPE ROLE AS ENUM('normie', 'admin');

CREATE TABLE IF NOT EXISTS Customers (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    wallet VARCHAR(42) UNIQUE NOT NULL,
    password VARCHAR(120) NOT NULL,
    role ROLE NOT NULL,
    verificationCode VARCHAR(10) NOT NULL,
    activated bool NOT NULL
);

CREATE TABLE IF NOT EXISTS PaymentInfo (
    customerEmail VARCHAR(255) PRIMARY KEY,
    callCount INT NOT NULL,
    subscription PLAN NOT NULL,
    planExpiration TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS Api (
    customerEmail VARCHAR(255),
    apiKey VARCHAR(255),
    PRIMARY KEY(customerEmail, apiKey)
);

CREATE TABLE IF NOT EXISTS Payments (    
    customerEmail VARCHAR(255) PRIMARY KEY,
    transactionHash VARCHAR(120) NOT NULL UNIQUE,
    asset ASSET NOT NULL, 
    amount BIGINT NOT NULL,
    chain CHAIN NOT NULL,
    date TIMESTAMPTZ NOT NULL
);
