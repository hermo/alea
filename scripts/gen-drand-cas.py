#!/usr/bin/env python3
"""
Regenerate src/drand_cas.h with Mozilla's root CA bundle.

Usage:
    python3 scripts/gen-drand-cas.py              # download bundle and regenerate
    python3 scripts/gen-drand-cas.py --check      # show live chain from api.drand.sh only
    python3 scripts/gen-drand-cas.py --bundle FILE # use a local PEM bundle instead
"""

import sys
import os
from datetime import datetime, timezone

BUNDLE_URL = "https://curl.se/ca/cacert.pem"
HOST = "api.drand.sh"
REPO_ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUTPUT = os.path.join(REPO_ROOT, "src", "drand_cas.h")


# --- PEM helpers ---

def split_pem_bundle(text):
    """Return list of individual PEM cert strings from a bundle."""
    pems, current = [], []
    for line in text.splitlines():
        if "-----BEGIN CERTIFICATE-----" in line:
            current = [line]
        elif "-----END CERTIFICATE-----" in line:
            current.append(line)
            pems.append("\n".join(current))
            current = []
        elif current:
            current.append(line)
    return pems


def fetch_live_chain_pems():
    import subprocess
    out = subprocess.run(
        ["openssl", "s_client", "-connect", f"{HOST}:443", "-showcerts"],
        input=b"", capture_output=True, timeout=15,
    ).stdout.decode()
    return split_pem_bundle(out)


def pem_to_der(pem):
    import subprocess
    r = subprocess.run(
        ["openssl", "x509", "-outform", "DER"],
        input=pem.encode(), capture_output=True, timeout=10,
    )
    return r.stdout if r.returncode == 0 else None


def cert_field(pem, *args):
    import subprocess
    out = subprocess.run(
        ["openssl", "x509"] + list(args),
        input=pem.encode(), capture_output=True, timeout=10,
    ).stdout.decode().strip()
    return out.split("=", 1)[-1].strip() if "=" in out else out


# --- Minimal DER parser ---

def _read_len(data, off):
    b = data[off]; off += 1
    if not (b & 0x80):
        return b, off
    n = b & 0x7f; v = 0
    for _ in range(n):
        v = (v << 8) | data[off]; off += 1
    return v, off


def _read_tlv(data, off):
    tag = data[off]; off += 1
    length, off = _read_len(data, off)
    return tag, bytes(data[off:off + length]), off + length


def _seq_items(data):
    off = 0
    while off < len(data):
        tag = data[off]; off += 1
        length, off = _read_len(data, off)
        yield tag, bytes(data[off:off + length])
        off += length


def _tbs_fields(der):
    """List of full TLV byte-strings from TBSCertificate."""
    _, cert_val, _ = _read_tlv(der, 0)
    _, tbs_val, _ = _read_tlv(cert_val, 0)
    fields = []
    off = 0
    while off < len(tbs_val):
        start = off
        tag = tbs_val[off]; off += 1
        length, off = _read_len(tbs_val, off)
        fields.append(bytes(tbs_val[start:off + length]))
        off += length
    return fields


def extract_subject_dn(der):
    fields = _tbs_fields(der)
    idx = 1 if fields[0][0] == 0xa0 else 0
    return fields[idx + 4]  # serial, sigalg, issuer, validity, subject


# OIDs
OID_RSA   = bytes([0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01])
OID_EC    = bytes([0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01])
OID_P256  = bytes([0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07])
OID_P384  = bytes([0x2b, 0x81, 0x04, 0x00, 0x22])
OID_P521  = bytes([0x2b, 0x81, 0x04, 0x00, 0x23])

BR_EC_CURVES = {OID_P256: 23, OID_P384: 24, OID_P521: 25}  # BearSSL curve IDs


def extract_pubkey(der):
    """
    Return one of:
      ('rsa', n_bytes, e_bytes)
      ('ec',  curve_id, q_bytes)
      ('unknown', None, None)
    """
    fields = _tbs_fields(der)
    idx = 1 if fields[0][0] == 0xa0 else 0
    spki_tlv = fields[idx + 5]

    _, spki_val, _ = _read_tlv(spki_tlv, 0)
    spki_items = list(_seq_items(spki_val))
    alg_items = list(_seq_items(spki_items[0][1]))
    alg_oid = alg_items[0][1]

    bit_val = spki_items[1][1][1:]  # skip unused-bits byte

    if alg_oid == OID_RSA:
        _, rsa_val, _ = _read_tlv(bit_val, 0)
        rsa_items = list(_seq_items(rsa_val))
        n = bytes(rsa_items[0][1])
        e = bytes(rsa_items[1][1])
        if n and n[0] == 0: n = n[1:]
        if e and e[0] == 0: e = e[1:]
        return ('rsa', n, e)

    if alg_oid == OID_EC:
        curve_oid = alg_items[1][1]
        curve_id = BR_EC_CURVES.get(curve_oid)
        if curve_id is None:
            return ('unknown', None, None)
        return ('ec', curve_id, bit_val)

    return ('unknown', None, None)


# --- C code generation ---

def c_array(name, data):
    hexb = [f"0x{b:02X}" for b in data]
    lines = [f"static const unsigned char {name}[] = {{"]
    for i in range(0, len(hexb), 12):
        chunk = hexb[i:i + 12]
        sep = "," if i + 12 < len(hexb) else ""
        lines.append("\t" + ", ".join(chunk) + sep)
    lines.append("};")
    return "\n".join(lines)


def c_ta_rsa(i, count):
    comma = "," if i < count - 1 else ""
    return "\n".join([
        "\t{",
        f"\t\t{{ (unsigned char *)TA{i}_DN, sizeof TA{i}_DN }},",
        "\t\tBR_X509_TA_CA,",
        "\t\t{ BR_KEYTYPE_RSA, { .rsa = {",
        f"\t\t\t(unsigned char *)TA{i}_RSA_N, sizeof TA{i}_RSA_N,",
        f"\t\t\t(unsigned char *)TA{i}_RSA_E, sizeof TA{i}_RSA_E,",
        "\t\t} } },",
        "\t}" + comma,
    ])


def c_ta_ec(i, curve_id, count):
    comma = "," if i < count - 1 else ""
    return "\n".join([
        "\t{",
        f"\t\t{{ (unsigned char *)TA{i}_DN, sizeof TA{i}_DN }},",
        "\t\tBR_X509_TA_CA,",
        f"\t\t{{ BR_KEYTYPE_EC, {{ .ec = {{ {curve_id},",
        f"\t\t\t(unsigned char *)TA{i}_EC_Q, sizeof TA{i}_EC_Q,",
        "\t\t} } },",
        "\t}" + comma,
    ])


def main():
    check_only = "--check" in sys.argv
    bundle_file = None
    if "--bundle" in sys.argv:
        idx = sys.argv.index("--bundle")
        bundle_file = sys.argv[idx + 1]

    if check_only:
        print(f"Live TLS chain from {HOST}:443:", file=sys.stderr)
        pems = fetch_live_chain_pems()
        for i, pem in enumerate(pems):
            subject   = cert_field(pem, "-noout", "-subject", "-nameopt", "oneline")
            issuer    = cert_field(pem, "-noout", "-issuer",  "-nameopt", "oneline")
            not_after = cert_field(pem, "-noout", "-enddate")
            role = "leaf" if i == 0 else ("root CA" if subject == issuer else "intermediate CA")
            print(f"  [{i}] {subject}", file=sys.stderr)
            print(f"       issuer:  {issuer}", file=sys.stderr)
            print(f"       expires: {not_after}  ({role})", file=sys.stderr)
        return 0

    if bundle_file:
        print(f"Reading bundle from {bundle_file} ...", file=sys.stderr)
        with open(bundle_file) as f:
            bundle_text = f.read()
        source_desc = bundle_file
    else:
        import subprocess
        print(f"Downloading Mozilla CA bundle from {BUNDLE_URL} ...", file=sys.stderr)
        r = subprocess.run(["curl", "-fsSL", BUNDLE_URL], capture_output=True, timeout=30)
        if r.returncode != 0:
            print(f"error: curl failed: {r.stderr.decode().strip()}", file=sys.stderr)
            sys.exit(1)
        bundle_text = r.stdout.decode("utf-8", errors="replace")
        source_desc = BUNDLE_URL

    pems = split_pem_bundle(bundle_text)
    print(f"Found {len(pems)} certificates in bundle.", file=sys.stderr)

    date = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    out = []
    out.append(f"/* Generated {date} by scripts/gen-drand-cas.py — do not edit by hand */")
    out.append(f"/* Source: {source_desc} */")
    out.append(f"/* Regenerate: python3 scripts/gen-drand-cas.py */")
    out.append("")

    tas = []  # list of (keytype, pem, arrays_text, struct_text)
    skipped = 0

    for pem in pems:
        der = pem_to_der(pem)
        if not der:
            skipped += 1
            continue

        try:
            dn = extract_subject_dn(der)
            keytype, a, b = extract_pubkey(der)
        except Exception:
            skipped += 1
            continue

        if keytype == 'unknown':
            skipped += 1
            continue

        tas.append((keytype, a, b, dn))

    print(f"Generating {len(tas)} trust anchors ({skipped} skipped).", file=sys.stderr)

    for i, (keytype, a, b, dn) in enumerate(tas):
        out.append(c_array(f"TA{i}_DN", dn))
        if keytype == 'rsa':
            out.append(c_array(f"TA{i}_RSA_N", a))
            out.append(c_array(f"TA{i}_RSA_E", b))
        else:  # ec
            out.append(c_array(f"TA{i}_EC_Q", b))
        out.append("")

    ta_count = len(tas)
    out.append(f"static const br_x509_trust_anchor TAs[{ta_count}] = {{")
    for i, (keytype, a, b, dn) in enumerate(tas):
        if keytype == 'rsa':
            out.append(c_ta_rsa(i, ta_count))
        else:
            out.append(c_ta_ec(i, a, ta_count))  # a = curve_id for EC
    out.append("};")
    out.append("")
    out.append(f"#define TAs_NUM   {ta_count}")
    out.append("")

    content = "\n".join(out)
    with open(OUTPUT, "w") as f:
        f.write(content)
    print(f"Wrote {OUTPUT} ({len(content)} bytes, {ta_count} trust anchors).", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
