pub fn disassemble(data: &[u8]) -> Vec<String> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect();
        out.push(format!("{:08x}: {:<48} |{}|", i * 16, hex.join(" "), ascii));
    }
    out
}

pub fn is_gcn_instruction(data: &[u8]) -> bool {
    // GCN instructions are 4 or 8 bytes, check alignment
    !data.is_empty() && data.len() % 4 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disasm_empty() {
        assert!(disassemble(&[]).is_empty());
    }

    #[test]
    fn disasm_produces_lines() {
        let lines = disassemble(&[0x00, 0x01, 0x02, 0x03]);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("00000000"));
    }

    #[test]
    fn gcn_check() {
        assert!(is_gcn_instruction(&[0u8; 4]));
        assert!(!is_gcn_instruction(&[0u8; 3]));
    }
}
