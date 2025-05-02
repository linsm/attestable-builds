use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use nsm_io::{AttestationDoc};
use serde_bytes::ByteBuf;
use openssl::x509::{X509, store::X509StoreBuilder, X509StoreContext};
use rustls_pki_types::{CertificateDer,pem::PemObject};
use webpki::{EndEntityCert, TrustAnchor, TlsServerTrustAnchors};
use openssl::stack::Stack;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::attestation_data::AttestationData;

pub(super) use aws_nitro_enclaves_cose::CoseSign1;
pub(super) use aws_nitro_enclaves_cose::crypto::Openssl;

static ROOT_CERT: &[u8] = include_bytes!("root.pem");

static SIGNATURE_ALGORITHM: &[&webpki::SignatureAlgorithm] = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::ED25519,
];

fn verify_pcrs(attestation_doc: &AttestationDoc, expected_pcrs: &[String]) {
    let mut pcr0 = String::new();
    let mut pcr1 = String::new();
    let mut pcr2 = String::new();

    attestation_doc.pcrs.iter().enumerate().for_each(|(i, pcr)| {        
        match i {
            0 => pcr0 = BASE64_STANDARD.encode(pcr.1.clone()),
            1 => pcr1 = BASE64_STANDARD.encode(pcr.1.clone()),
            2 => pcr2 = BASE64_STANDARD.encode(pcr.1.clone()),
            _ => {}
        }
    });
    assert_eq!(pcr0, expected_pcrs[0], "PCR0 mismatch");
    assert_eq!(pcr1, expected_pcrs[1], "PCR1 mismatch");
    assert_eq!(pcr2, expected_pcrs[2], "PCR2 mismatch");
}

// TODO: Refactor user_data 
// Currently array should be formated like: commit_hash,artifact_name,artifact_hash
fn verify_user_dat(attestation_doc: &AttestationDoc, expected_user_data: &[String]) -> Result<(), anyhow::Error> {
    let user_data_buf : ByteBuf = attestation_doc.user_data.as_ref().ok_or_else(|| anyhow::anyhow!("User data not found"))?.clone();    
    let user_data = String::from_utf8(user_data_buf.into_vec()).map_err(|_| anyhow::anyhow!("Failed to parse user data"))?;
    let data_parts: Vec<&str> = user_data.split(',').collect();
    for (i,part) in data_parts.iter().enumerate() {
        let parts: Vec<&str> = part.split('=').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid user data format"));
        }
        let _key = parts[0];
        let value = parts[1];
        assert_eq!(value, expected_user_data[i], "User data mismatch");
    }
    Ok(())
}

fn verify_signature_rustls(attestation_doc: &AttestationDoc) -> Result<(), anyhow::Error> {

    // Prepare trust anchor    
    let x = CertificateDer::from_pem_slice(ROOT_CERT).unwrap();
    let trust_anchor = [TrustAnchor::try_from_cert_der(&x).map_err(|_| anyhow::anyhow!("Failed"))?];    
    let trust_anchors = TlsServerTrustAnchors(&trust_anchor);

    // Prepare intermediate certificates
    let mut intermediate_certs = Vec::new();
    let ca_bundle = &attestation_doc.cabundle;   
    ca_bundle.iter().for_each(|cert| {
        let c = cert.as_slice();
        intermediate_certs.push(c);
    });

    // Prepare EndEntity certificate
    let certa = &attestation_doc.certificate.to_vec();
    let end_entity_cert = EndEntityCert::try_from(certa.as_slice()).map_err(|_| anyhow::anyhow!("Failed to create end entity cert"))?;    
    
    let epoc = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let epoch = webpki::Time::from_seconds_since_unix_epoch(epoc.as_secs() as u64);

    let verification_result = end_entity_cert.verify_is_valid_tls_server_cert(
        SIGNATURE_ALGORITHM,
        &trust_anchors,
        &intermediate_certs,
        epoch,
    );

    match verification_result {
        Ok(_) => println!("Certificate verification succeeded"),
        Err(e) => println!("Certificate verification failed: {:?}", e),
    }
    //TODO insert assert
    Ok(())
}

fn verify_signature_openssl(attestation_doc: &AttestationDoc) -> Result<(), anyhow::Error> {

    // trusted x509 store
    let mut trusted_builder = X509StoreBuilder::new()?;
    let root_cert = X509::from_pem(ROOT_CERT).map_err(|_| anyhow::anyhow!("Failed to parse root cert"))?;
    trusted_builder.add_cert(root_cert.clone()).map_err(|_| anyhow::anyhow!("Failed to add root cert"))?;
    //trusted_builder.set_flags(X509VerifyFlags::NO_CHECK_TIME)?;
    let trusted_store = trusted_builder.build();

    //certificate to be verified   
    let cert_to_be_verified = X509::from_der(&attestation_doc.certificate).map_err(|_| anyhow::anyhow!("Failed to parse certificate"))?;
    //println!("Certificate to be verified: {:?}", cert_to_be_verified);

    //certificate chain
    let mut intermediate_certs = Stack::new()?;
    let ca_bundle = &attestation_doc.cabundle;   
    ca_bundle.iter().for_each(|cert| {
        let cert: X509 = X509::from_der(cert).unwrap();
        let _ = intermediate_certs.push(cert.clone());
    });
        
    // Init X509 Context
    let mut store_context = X509StoreContext::new()?;    
    store_context.init(&trusted_store, &cert_to_be_verified, &intermediate_certs, |context| {
        let verify_result = context.verify_cert();
        match verify_result {
            Ok(ref result) => println!("Certificate verification succeeded: {:?}", result),
            Err(ref e) => println!("Certificate verification failed: {:?}", e),
        }
        Ok(())            
    })?;

    Ok(())
}

pub async fn validate_attestation_document(    
    attestation_data: AttestationData,    
) -> anyhow::Result<()> {

    let at = BASE64_STANDARD.decode(attestation_data.attestation_document).map_err(|_| anyhow::anyhow!("Failed to decode attestation document"))?;
    let cose_sign_1 = CoseSign1::from_bytes(&at)?;
    let payload = cose_sign_1.get_payload::<Openssl>(None).unwrap();    
    let attestation_doc: AttestationDoc = serde_cbor::from_slice(&payload)?;
    let cert_to_be_verified = X509::from_der(&attestation_doc.certificate).map_err(|_| anyhow::anyhow!("Failed to parse certificate"))?;    
    let _signature = cose_sign_1.verify_signature::<Openssl>(&cert_to_be_verified.public_key()?).map_err(|_| anyhow::anyhow!("Failed to verify signature"))?;     
    //TODO verify signature

    verify_pcrs(&attestation_doc, &[attestation_data.pcr0, attestation_data.pcr1, attestation_data.pcr2]);
    let _ =verify_signature_rustls(&attestation_doc);
    let _ =verify_signature_openssl(&attestation_doc);
    verify_user_dat(&attestation_doc, &[attestation_data.commit_hash, attestation_data.artifact_name, attestation_data.artifact_hash])?;
   
    Ok(())
}

