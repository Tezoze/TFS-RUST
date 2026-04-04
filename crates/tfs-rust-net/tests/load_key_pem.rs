#[test]
fn workspace_key_pem_loads() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../key.pem");
    let pem = std::fs::read_to_string(&path).expect("read key.pem");
    tfs_rust_net::rsa::private_key_from_pkcs1_pem(&pem).expect("relaxed PEM load");
}
