use anyhow::Result;
use simd_json::{self, OwnedValue};
use ahash::AHashMap;

/// Ultra-high performance message parser using SIMD JSON
pub struct FastMessageParser {
    // Pre-allocated buffer for SIMD parsing
    buffer: Vec<u8>,
    // Cache for frequently accessed fields
    field_cache: AHashMap<String, OwnedValue>,
}

impl FastMessageParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(16384), // 16KB buffer
            field_cache: AHashMap::with_capacity(64),
        }
    }

    /// Parse JSON message using SIMD optimizations
    pub fn parse_fast(&mut self, text: &str) -> Result<OwnedValue> {
        // Clear and resize buffer if needed
        self.buffer.clear();
        self.buffer.extend_from_slice(text.as_bytes());
        
        // Use SIMD JSON for ultra-fast parsing
        match simd_json::to_owned_value(&mut self.buffer) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Fallback to standard JSON parser
                tracing::warn!("SIMD JSON parsing failed, falling back to std: {}", e);
                let value: serde_json::Value = serde_json::from_str(text)?;
                Ok(simd_json::to_owned_value(&mut value.to_string().into_bytes())?)
            }
        }
    }

    /// Fast channel detection using byte pattern matching
    #[inline(always)]
    pub fn detect_channel_fast(data: &[u8]) -> Option<&'static str> {
        // Use SIMD-style byte pattern matching for ultra-fast channel detection
        if data.len() < 10 { return None; } // Minimum length for a meaningful JSON message
        
        // Look for "level3" pattern
        if memmem_fast(data, b"level3") {
            return Some("level3");
        }
        
        // Look for "trade" pattern  
        if memmem_fast(data, b"trade") {
            return Some("trade");
        }
        
        // Look for "balances" pattern
        if memmem_fast(data, b"balances") {
            return Some("balances");
        }
        
        None
    }

    /// Fast Level3 update detection using SIMD byte scanning
    #[inline(always)]
    pub fn is_level3_update(&self, data: &[u8]) -> bool {
        memmem_fast(data, b"level3") && memmem_fast(data, b"update")
    }

    /// Parse Level3 update with minimal allocations
    pub fn parse_level3_update_fast(&self, text: &str) -> Result<crate::handlers::models::kraken::level3_update::Level3Update> {
        // For critical path, use standard serde_json to avoid mutable buffer requirement
        let update: crate::handlers::models::kraken::level3_update::Level3Update = 
            serde_json::from_str(text)?;
        Ok(update)
    }

    /// Extract symbol from message using fast byte scanning
    #[inline(always)]
    pub fn extract_symbol_fast(&self, data: &[u8]) -> Option<String> {
        // Look for "symbol":"BTC/USD" pattern
        if let Some(pos) = self.find_pattern(data, b"\"symbol\":\"") {
            let start = pos + 10; // Length of "symbol":"
            if let Some(end_pos) = self.find_quote_end(&data[start..]) {
                if let Ok(symbol) = std::str::from_utf8(&data[start..start + end_pos]) {
                    return Some(symbol.to_string());
                }
            }
        }
        None
    }

    /// Find pattern in byte array with SIMD-style scanning
    #[inline(always)]
    fn find_pattern(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if memmem_fast(haystack, needle) {
            // Linear search for exact position - could be optimized further
            for i in 0..=haystack.len().saturating_sub(needle.len()) {
                if &haystack[i..i + needle.len()] == needle {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Find end quote position
    #[inline(always)]
    fn find_quote_end(&self, data: &[u8]) -> Option<usize> {
        for (i, &byte) in data.iter().enumerate() {
            if byte == b'"' {
                return Some(i);
            }
        }
        None
    }
}

/// Ultra-fast memory search using manual SIMD-style operations
#[inline(always)]
fn memmem_fast(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }

    let first = needle[0];
    let mut pos = 0;

    while pos <= haystack.len() - needle.len() {
        // Fast path: check first byte
        if haystack[pos] == first {
            // Check remaining bytes in chunks of 8 for better cache utilization
            let mut match_found = true;
            let chunks = needle.len() / 8;
            let remainder = needle.len() % 8;
            
            // Process 8-byte chunks
            for i in 0..chunks {
                let offset = pos + i * 8;
                if offset + 8 <= haystack.len() {
                    let haystack_chunk = &haystack[offset..offset + 8];
                    let needle_chunk = &needle[i * 8..(i + 1) * 8];
                    if haystack_chunk != needle_chunk {
                        match_found = false;
                        break;
                    }
                }
            }
            
            // Process remaining bytes
            if match_found && remainder > 0 {
                let offset = pos + chunks * 8;
                for i in 0..remainder {
                    if haystack[offset + i] != needle[chunks * 8 + i] {
                        match_found = false;
                        break;
                    }
                }
            }
            
            if match_found {
                return true;
            }
        }
        pos += 1;
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_channel_detection() {
        assert_eq!(
            FastMessageParser::detect_channel_fast(b"{\"channel\":\"level3\"}"),
            Some("level3")
        );
        assert_eq!(
            FastMessageParser::detect_channel_fast(b"{\"channel\":\"trade\"}"),
            Some("trade")
        );
    }

    #[test] 
    fn test_memmem_fast() {
        assert!(memmem_fast(b"hello world level3 data", b"level3"));
        assert!(!memmem_fast(b"hello world", b"level3"));
    }
}
