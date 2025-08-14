use ethereum_types::U256;

/// Convert bytes to U256, padding with zeros if necessary
pub fn bytes_to_u256(bytes: &[u8]) -> U256 {
    let mut padded = [0u8; 32];
    let start = if bytes.len() > 32 { 0 } else { 32 - bytes.len() };
    let end = start + bytes.len().min(32);
    padded[start..end].copy_from_slice(&bytes[..bytes.len().min(32)]);
    U256::from_big_endian(&padded)
}

/// Convert U256 to bytes
pub fn u256_to_bytes(value: U256) -> Vec<u8> {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    bytes.to_vec()
}

/// Convert U256 to a 32-byte array
pub fn u256_to_bytes32(value: U256) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    bytes
}

/// Safely resize a vector to a given size
pub fn safe_resize(vec: &mut Vec<u8>, new_size: usize, max_size: usize) -> Result<(), String> {
    if new_size > max_size {
        return Err(format!("Size {} exceeds maximum {}", new_size, max_size));
    }
    vec.resize(new_size, 0);
    Ok(())
}

/// Format a U256 as a hex string with 0x prefix
pub fn format_hex_u256(value: U256) -> String {
    format!("0x{:x}", value)
}

/// Format bytes as a hex string with 0x prefix
pub fn format_hex_bytes(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::U256;

    #[test]
    fn test_bytes_to_u256() {
        let bytes = vec![0x01, 0x23, 0x45];
        let result = bytes_to_u256(&bytes);
        assert_eq!(result, U256::from(0x012345));
    }

    #[test]
    fn test_u256_to_bytes() {
        let value = U256::from(0x012345);
        let bytes = u256_to_bytes(value);
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[29..], &[0x01, 0x23, 0x45]);
    }

    #[test]
    fn test_safe_resize() {
        let mut vec = vec![1, 2, 3];
        assert!(safe_resize(&mut vec, 5, 10).is_ok());
        assert_eq!(vec.len(), 5);
        assert_eq!(vec, vec![1, 2, 3, 0, 0]);
        
        let mut vec2 = vec![1, 2, 3];
        assert!(safe_resize(&mut vec2, 15, 10).is_err());
    }

    #[test]
    fn test_format_hex() {
        let value = U256::from(0x123456);
        assert_eq!(format_hex_u256(value), "0x123456");
        
        let bytes = vec![0x12, 0x34, 0x56];
        assert_eq!(format_hex_bytes(&bytes), "0x123456");
    }
}
