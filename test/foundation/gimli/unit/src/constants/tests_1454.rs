#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dw_eh_pe_format() {
        let encoding = DW_EH_PE_pcrel | DW_EH_PE_uleb128;
        assert_eq!(encoding.format(), DW_EH_PE_uleb128);
    }

    #[test]
    fn test_dw_eh_pe_application() {
        let encoding = DW_EH_PE_pcrel | DW_EH_PE_uleb128;
        assert_eq!(encoding.application(), DW_EH_PE_pcrel);
    }

    #[test]
    fn test_dw_eh_pe_is_absent() {
        assert!(!DW_EH_PE_absptr.is_absent());
        assert!(DW_EH_PE_omit.is_absent());
    }

    #[test]
    fn test_dw_eh_pe_is_valid_encoding_ok() {
        let encoding = DW_EH_PE_uleb128 | DW_EH_PE_pcrel;
        assert!(encoding.is_valid_encoding());
        assert!(DW_EH_PE_absptr.is_valid_encoding());
        assert!(DW_EH_PE_omit.is_valid_encoding());
    }

    #[test]
    fn test_dw_eh_pe_is_valid_encoding_bad_format() {
        let encoding = DwEhPe((DW_EH_PE_sdata8.0 + 1) | DW_EH_PE_pcrel.0);
        assert!(!encoding.is_valid_encoding());
    }

    #[test]
    fn test_dw_eh_pe_is_valid_encoding_bad_application() {
        let encoding = DwEhPe(DW_EH_PE_sdata8.0 | (DW_EH_PE_aligned.0 + 1));
        assert!(!encoding.is_valid_encoding());
    }
}
