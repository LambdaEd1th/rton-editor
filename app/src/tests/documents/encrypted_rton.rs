use super::super::*;
use rton_editor_core::ByteRead;

#[test]
fn encrypted_rton_tabs_show_decrypted_hex_bytes() {
    let doc = parse_text(SAMPLE_JSON, TextFormat::Json).expect("sample parses");
    let plain = encode_rton_bytes(&doc.value, EncodeOptions::default()).expect("rton encodes");
    let encrypted = encode_rton_bytes(
        &doc.value,
        EncodeOptions {
            encoding: BinaryEncoding::Standard,
            encrypted: true,
        },
    )
    .expect("encrypted rton encodes");

    let tab =
        create_tab_from_bytes(17, "encrypted.rton".to_string(), &encrypted).expect("tab opens");

    assert_eq!(tab.mode, EditorMode::RtonHex);
    assert_eq!(
        tab.byte_doc.as_ref().map(|byte_doc| byte_doc.to_vec()),
        Some(plain.clone())
    );
    assert!(
        !tab.byte_doc
            .as_ref()
            .expect("hex bytes")
            .starts_with_bytes(ENCRYPTED_RTON_PREFIX)
    );
    assert_eq!(
        document_for_tab(&tab)
            .expect("decrypted hex bytes parse")
            .value,
        doc.value
    );
    assert_eq!(
        locate_rton_value_offset(tab.byte_doc.as_ref().expect("hex bytes"), "$"),
        Some(8)
    );
}
