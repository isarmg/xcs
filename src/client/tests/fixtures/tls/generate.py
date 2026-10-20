#!/usr/bin/env python3
"""Regenerate synthetic local-only TLS fixtures (requires Python cryptography).

Tests consume the committed DER files and do not run this script or require
Python, OpenSSL, network access, a registry dependency, or a real CA private key.
The generated server key is intentionally public test data, never a credential.
"""

from datetime import datetime, timezone
from ipaddress import ip_address
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

HERE = Path(__file__).resolve().parent
START = datetime(2020, 1, 1, tzinfo=timezone.utc)
END = datetime(2120, 1, 1, tzinfo=timezone.utc)


def name(common_name):
    return x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, common_name)])


def certificate(subject, issuer, key, signer, serial, ca):
    builder = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer)
        .public_key(key.public_key())
        .serial_number(serial)
        .not_valid_before(START)
        .not_valid_after(END)
        .add_extension(x509.BasicConstraints(ca=ca, path_length=None), critical=True)
        .add_extension(
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=False,
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=ca,
                crl_sign=ca,
                encipher_only=None,
                decipher_only=None,
            ),
            critical=True,
        )
    )
    if not ca:
        builder = builder.add_extension(
            x509.SubjectAlternativeName(
                [x509.DNSName("localhost"), x509.IPAddress(ip_address("127.0.0.1"))]
            ),
            critical=False,
        ).add_extension(
            x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False
        )
    return builder.sign(signer, hashes.SHA256())


root_key, intermediate_key, leaf_key = [
    ec.generate_private_key(ec.SECP256R1()) for _ in range(3)
]
root_name = name("ZXS regression root - NOT FOR PRODUCTION")
intermediate_name = name("ZXS regression intermediate - NOT FOR PRODUCTION")
for filename, cert in [
    ("root.der", certificate(root_name, root_name, root_key, root_key, 1, True)),
    (
        "intermediate.der",
        certificate(intermediate_name, root_name, intermediate_key, root_key, 2, True),
    ),
    (
        "localhost.der",
        certificate(name("localhost"), intermediate_name, leaf_key, intermediate_key, 3, False),
    ),
]:
    (HERE / filename).write_bytes(cert.public_bytes(serialization.Encoding.DER))

(HERE / "localhost-test-key.pkcs8.der").write_bytes(
    leaf_key.private_bytes(
        serialization.Encoding.DER,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )
)
