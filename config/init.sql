-- DataEngine Database Initialization Script
-- This script initializes the database with TimescaleDB extension and basic data

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Create database user if not exists
DO
$$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_user WHERE usename = 'dataengine') THEN
        CREATE USER dataengine WITH PASSWORD 'dataengine123';
    END IF;
END
$$;

-- Grant necessary permissions
GRANT ALL PRIVILEGES ON DATABASE dataengine TO dataengine;
GRANT ALL ON SCHEMA public TO dataengine;
GRANT ALL ON ALL TABLES IN SCHEMA public TO dataengine;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO dataengine;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO dataengine;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO dataengine;

-- Create test exchanges
INSERT INTO exchanges (exchange, country, timezone, trading_hours_start, trading_hours_end) 
VALUES 
    ('KRAKEN', 'US', 'UTC', '00:00:00', '23:59:59'),
    ('BINANCE', 'US', 'UTC', '00:00:00', '23:59:59')
ON CONFLICT (exchange) DO NOTHING;

-- Create test securities
INSERT INTO securities (symbol, exchange, asset_class, currency, tick_size, lot_size, contract_size) 
VALUES 
    ('BTC/USD', 'KRAKEN', 'cryptocurrency', 'USD', 0.01, 0.001, 1.0),
    ('ETH/USD', 'KRAKEN', 'cryptocurrency', 'USD', 0.01, 0.001, 1.0),
    ('BTC/USDT', 'BINANCE', 'cryptocurrency', 'USDT', 0.01, 0.001, 1.0),
    ('ETH/USDT', 'BINANCE', 'cryptocurrency', 'USDT', 0.01, 0.001, 1.0)
ON CONFLICT (symbol) DO NOTHING;

-- Create initial order books for test symbols
INSERT INTO order_books (symbol, exchange, security_id, exchange_id, total_volume)
SELECT 
    s.symbol, 
    s.exchange,
    s.security_id,
    e.exchange_id,
    0.0
FROM securities s
JOIN exchanges e ON s.exchange = e.exchange
ON CONFLICT (symbol) DO NOTHING;

-- Add some sample historical data for testing
INSERT INTO trades (symbol, exchange, trade_id, security_id, exchange_id, side, price, quantity, created_at)
SELECT 
    s.symbol,
    s.exchange,
    'test_trade_' || generate_series(1, 10),
    s.security_id,
    e.exchange_id,
    CASE WHEN random() > 0.5 THEN 'buy' ELSE 'sell' END,
    50000 + (random() * 10000), -- Random price between 50k-60k
    random() * 0.1, -- Random quantity up to 0.1
    now() - (random() * interval '1 hour')
FROM securities s
JOIN exchanges e ON s.exchange = e.exchange
WHERE s.symbol = 'BTC/USD'
ON CONFLICT DO NOTHING;

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_trades_created_at ON trades (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_trades_symbol_time ON trades (symbol, created_at DESC);

-- Display initialization results
SELECT 'Database initialized successfully!' as status;
SELECT COUNT(*) as exchange_count FROM exchanges;
SELECT COUNT(*) as securities_count FROM securities;
SELECT COUNT(*) as order_books_count FROM order_books;
SELECT COUNT(*) as sample_trades_count FROM trades;
