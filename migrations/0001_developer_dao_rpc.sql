CREATE TYPE PLAN AS ENUM('based', 'premier', 'gigachad');

CREATE TABLE IF NOT EXISTS Customers (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    wallet VARCHAR(42) NOT NULL,
    password VARCHAR(120) NOT NULL
);

CREATE TABLE IF NOT EXISTS Payments (
    customerEmail VARCHAR(255) PRIMARY KEY,
    callCount int NOT NULL,
    subscription PLAN NOT NULL,
    paymentDate TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS Api (
    customerEmail VARCHAR(255),
    apiKey VARCHAR(255),
    PRIMARY KEY(customerEmail, apiKey)
);
