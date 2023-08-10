--Please write your up migrations here

CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    description TEXT,
  
    created_at TIMESTAMP DEFAULT NOW()
);