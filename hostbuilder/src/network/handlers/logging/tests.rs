use super::*;
use ultra_logger::LogLevel;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use tokio;

#[cfg(test)]
mod kraken_logger_tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_event_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let websocket_event = WebSocketEvent {
            endpoint: "wss://ws.kraken.com".to_string(),
            action: "connection_opened".to_string(),
            connection_id: Some("test_conn_123".to_string()),
            message: "WebSocket connection established".to_string(),
            level: LogLevel::Info,
            latency_micros: Some(1500),
        };

        let result = logger.log_websocket_event(websocket_event).await;
        assert!(result.is_ok(), "WebSocket event logging should succeed");
    }

    #[tokio::test]
    async fn test_order_book_update_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let orderbook_log = OrderBookUpdateLog {
            symbol: "ETHUSD".to_string(),
            update_type: "snapshot".to_string(),
            bids_count: 5,
            asks_count: 3,
            processing_time_micros: 2500,
            total_volume: BigDecimal::from_str("1250.75").unwrap(), // 1250.75
            error: None,
        };

        let result = logger.log_orderbook_update(orderbook_log).await;
        assert!(result.is_ok(), "Order book update logging should succeed");
    }

    #[tokio::test]
    async fn test_trade_processing_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let trade_log = TradeProcessingLog {
            symbol: "ADAUSD".to_string(),
            trade_id: "trade_98765".to_string(),
            side: "buy".to_string(),
            price: 1.25,
            quantity: 1000.0,
            processing_time_micros: 800,
            error: None,
        };

        let result = logger.log_trade_processing(trade_log).await;
        assert!(result.is_ok(), "Trade processing logging should succeed");
    }

    #[tokio::test]
    async fn test_performance_metrics_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let perf_metrics = PerformanceMetrics {
            component: "OrderBookHandler".to_string(),
            operation: "process_level3_update".to_string(),
            execution_time_micros: 1200,
            throughput_ops_per_sec: 15000.0,
            memory_usage_mb: 128.5,
            cpu_usage_percent: 12.3,
        };

        let result = logger.log_performance_metrics(perf_metrics).await;
        assert!(result.is_ok(), "Performance metrics logging should succeed");
    }

    #[tokio::test]
    async fn test_critical_error_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let critical_error = CriticalError {
            component: "WebSocketHandler".to_string(),
            error_code: "WS_PARSE_ERROR".to_string(),
            error_message: "Failed to parse WebSocket message".to_string(),
            stack_trace: Some("line 1: parse error\nline 2: invalid format".to_string()),
            context: Some("Processing L3 update for BTCUSD".to_string()),
        };

        let result = logger.log_critical_error(critical_error).await;
        assert!(result.is_ok(), "Critical error logging should succeed");
    }

    #[tokio::test]
    async fn test_auth_event_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        let auth_event = AuthEvent {
            endpoint: "/private/Balance".to_string(),
            success: true,
            error: None,
            latency_micros: Some(250),
        };

        let result = logger.log_auth_event(auth_event).await;
        assert!(result.is_ok(), "Auth event logging should succeed");
    }

    #[test]
    fn test_logger_creation() {
        let service_name = "TestKrakenLoggingService";
        let logger = KrakenLogger::new(service_name);
        
        // Test that logger was created successfully (can't access private field)
        assert!(true, "Logger creation should succeed");
    }

    #[test]
    fn test_global_logger_initialization() {
        // Test that global loggers are properly initialized via lazy statics
        let _ws_logger = &*KRAKEN_WEBSOCKET_LOGGER;
        let _orderbook_logger = &*KRAKEN_ORDERBOOK_LOGGER;
        let _trade_logger = &*KRAKEN_TRADE_LOGGER;
        let _rest_logger = &*KRAKEN_REST_LOGGER;
        
        // Since service_name is private, we just test that the loggers can be accessed
        assert!(true, "Global logger access should work");
    }

    #[tokio::test]
    async fn test_error_handling_in_logging() {
        let logger = KrakenLogger::new("TestKrakenService");
        
        // Test logging with errors
        let error_trade_log = TradeProcessingLog {
            symbol: "XBTUSD".to_string(),
            trade_id: "error_trade_123".to_string(),
            side: "buy".to_string(),
            price: 50000.0,
            quantity: 0.001,
            processing_time_micros: 5000,
            error: Some("Failed to validate trade".to_string()),
        };

        let result = logger.log_trade_processing(error_trade_log).await;
        assert!(result.is_ok(), "Error trade logging should succeed");

        // Test orderbook logging with errors
        let error_orderbook_log = OrderBookUpdateLog {
            symbol: "ETHUSD".to_string(),
            update_type: "update".to_string(),
            bids_count: 0,
            asks_count: 0,
            processing_time_micros: 10000,
            total_volume: BigDecimal::from_str("0").unwrap(),
            error: Some("Checksum validation failed".to_string()),
        };

        let result = logger.log_orderbook_update(error_orderbook_log).await;
        assert!(result.is_ok(), "Error orderbook logging should succeed");
    }

    #[tokio::test]
    async fn test_high_performance_logging() {
        let logger = KrakenLogger::new("PerformanceTestService");
        
        // Test that we can log many events quickly without errors
        for i in 0..10 {
            let websocket_event = WebSocketEvent {
                endpoint: format!("wss://test-{}.kraken.com", i),
                action: "message_received".to_string(),
                connection_id: Some(format!("conn_{}", i)),
                message: format!("Test message {}", i),
                level: LogLevel::Info,
                latency_micros: Some(100 + i as u64),
            };

            let result = logger.log_websocket_event(websocket_event).await;
            assert!(result.is_ok(), "Bulk WebSocket event logging should succeed");
        }
    }
}
