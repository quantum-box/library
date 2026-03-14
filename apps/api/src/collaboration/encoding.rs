/// lib0 variable-length encoding compatible with y-protocols.
///
/// The y-websocket protocol uses lib0 encoding for message
/// framing. Values 0-127 encode as a single byte.

/// Read a variable-length unsigned integer from `buf` starting
/// at `pos`. Advances `pos` past the consumed bytes.
pub fn read_var_uint(buf: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= buf.len() {
            break;
        }
        let byte = buf[*pos] as u64;
        *pos += 1;
        result |= (byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

/// Write a variable-length unsigned integer into `buf`.
pub fn write_var_uint(buf: &mut Vec<u8>, mut num: u64) {
    loop {
        if num > 0x7f {
            buf.push(0x80 | (num & 0x7f) as u8);
            num >>= 7;
        } else {
            buf.push(num as u8);
            break;
        }
    }
}

/// Read a variable-length byte array (length-prefixed) from `buf`.
pub fn read_var_bytes(buf: &[u8], pos: &mut usize) -> Vec<u8> {
    let len = read_var_uint(buf, pos) as usize;
    let end = (*pos + len).min(buf.len());
    let data = buf[*pos..end].to_vec();
    *pos = end;
    data
}

/// Write a variable-length byte array (length-prefixed) into `buf`.
pub fn write_var_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    write_var_uint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_var_uint_small() {
        let mut buf = Vec::new();
        write_var_uint(&mut buf, 42);
        let mut pos = 0;
        assert_eq!(read_var_uint(&buf, &mut pos), 42);
        assert_eq!(pos, 1);
    }

    #[test]
    fn roundtrip_var_uint_large() {
        let mut buf = Vec::new();
        write_var_uint(&mut buf, 300);
        let mut pos = 0;
        assert_eq!(read_var_uint(&buf, &mut pos), 300);
        assert_eq!(pos, 2);
    }

    #[test]
    fn roundtrip_var_bytes() {
        let mut buf = Vec::new();
        write_var_bytes(&mut buf, b"hello");
        let mut pos = 0;
        assert_eq!(read_var_bytes(&buf, &mut pos), b"hello");
    }
}
