use rcgen::*;

pub async fn gen_tls(domain: impl Into<String>, ip: impl Into<String>) -> (Vec<u8>,Vec<u8>){
    let subject_alt_names = vec![
        domain.into(),
        ip.into()
    ];
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names).expect("Erro ao Criar Chave TLS");
    let cert_vec = cert.pem().as_bytes().to_vec();
    let key_vec = signing_key.serialize_pem().as_bytes().to_vec();
    (cert_vec,key_vec)
}