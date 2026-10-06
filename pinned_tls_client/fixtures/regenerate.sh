#!/bin/sh
# Regenerates the test certificate authorities and keys beside this script. Run once, by hand,
# and commit the output; nothing in a gate runs it, so a test never depends on the host's openssl.
#
# **Every key here is a test fixture and protects nothing.** These are two throwaway certificate
# authorities that exist so a test can show the client trusting exactly one root: `pinned` is the
# root a test pins, and `stranger` is a well-formed authority the client was never given. Neither
# is trusted by anything outside this tree's tests, and publishing the private keys is the point:
# helpers/tls-peer must be able to sign a handshake with them.
#
# The shapes copy what Let's Encrypt actually serves, measured 2026-10-06 (UTC) against
# letsencrypt.org: a P-384 root, a P-384 intermediate, and a P-256 leaf whose certificate is signed
# with ECDSA over SHA-384. So the chain walk exercises the same signature algorithms the real
# index host's chain will.
#
# Validity runs from 2026-01-01 to 2036-01-01 (UTC), because a nife guest has no clock a stranger
# can trust (milestone 501 (a TLS client that speaks to one pinned peer)'s BUGS) and a fixture that expired mid-decade would read as a regression.
#
# Names use `.test`, which RFC 6761 reserves, so no fixture can be mistaken for a real host.
#
#   pinned-root.der              P-384 root, the one the tests pin
#   pinned-intermediate.der      P-384, issued by pinned-root
#   basalt-test-chain.pem        leaf for basalt.test then the intermediate (what the peer sends)
#   basalt-test-key.pem          the leaf's P-256 key
#   stranger-root.der            P-384 root that nothing pins, the same shape as the pinned one, so
#                                a refusal can only be about which root it is
#   stranger-test-chain.pem      leaf for stranger.test issued by stranger-root
#   stranger-test-key.pem        its key
set -eu
cd "$(dirname "$0")"
work="$(mktemp -d "${TMPDIR:-/tmp}/pinned-tls-fixtures.XXXXXX")"
trap 'rm -rf "$work"' EXIT

start=20260101000000Z
end=20360101000000Z

ca_ext="basicConstraints=critical,CA:TRUE
keyUsage=critical,keyCertSign,cRLSign
subjectKeyIdentifier=hash"

leaf_ext() {
    printf 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:%s\n' "$1"
}

# `openssl x509 -req` cannot set both validity dates; `openssl ca` can, with a throwaway database.
# A root is signed by its own key, which is `-selfsign` and no issuer certificate.
sign() { # sign <csr> <issuer-cert or "self"> <issuer-key> <ext-file> <out>
    db="$work/db"; rm -rf "$db"; mkdir -p "$db"; : >"$db/index.txt"; echo 1000 >"$db/serial"
    printf '[ca]\ndefault_ca=d\n[d]\ndatabase=%s/index.txt\nserial=%s/serial\nnew_certs_dir=%s\npolicy=p\nunique_subject=no\n[p]\ncommonName=supplied\n' \
        "$db" "$db" "$db" >"$work/ca.cnf"
    if [ "$2" = self ]; then issuer="-selfsign"; else issuer="-cert $2"; fi
    # shellcheck disable=SC2086
    openssl ca -batch -config "$work/ca.cnf" -in "$1" $issuer -keyfile "$3" -extfile "$4" \
        -startdate "$start" -enddate "$end" -md sha384 -notext -out "$5" 2>"$work/ca.log" ||
        { cat "$work/ca.log" >&2; exit 1; }
}

self_signed_root() { # <curve> <cn> <stem>
    openssl ecparam -name "$1" -genkey -noout -out "$work/$3.key"
    openssl req -new -key "$work/$3.key" -subj "/CN=$2" -out "$work/$3.csr"
    printf '%s\n' "$ca_ext" >"$work/$3.ext"
    sign "$work/$3.csr" self "$work/$3.key" "$work/$3.ext" "$work/$3.pem"
}

self_signed_root secp384r1 "nife test root, pinned (not a real certificate authority)" pinned-root
self_signed_root secp384r1 "nife test root, stranger (not a real certificate authority)" stranger-root

# The pinned chain's intermediate.
openssl ecparam -name secp384r1 -genkey -noout -out "$work/pinned-intermediate.key"
openssl req -new -key "$work/pinned-intermediate.key" -subj "/CN=nife test intermediate (not a real certificate authority)" -out "$work/pinned-intermediate.csr"
printf '%s\n' "$ca_ext" >"$work/ca.ext"
sign "$work/pinned-intermediate.csr" "$work/pinned-root.pem" "$work/pinned-root.key" "$work/ca.ext" "$work/pinned-intermediate.pem"

leaf() { # <name> <issuer-stem> <out-stem>
    openssl ecparam -name prime256v1 -genkey -noout -out "$work/$3.key"
    openssl req -new -key "$work/$3.key" -subj "/CN=$1" -out "$work/$3.csr"
    leaf_ext "$1" >"$work/$3.ext"
    sign "$work/$3.csr" "$work/$2.pem" "$work/$2.key" "$work/$3.ext" "$work/$3.pem"
}

leaf basalt.test pinned-intermediate basalt-test
leaf stranger.test stranger-root stranger-test

openssl x509 -in "$work/pinned-root.pem" -outform der -out pinned-root.der
openssl x509 -in "$work/pinned-intermediate.pem" -outform der -out pinned-intermediate.der
openssl x509 -in "$work/stranger-root.pem" -outform der -out stranger-root.der
cat "$work/basalt-test.pem" "$work/pinned-intermediate.pem" >basalt-test-chain.pem
cat "$work/stranger-test.pem" >stranger-test-chain.pem
openssl pkcs8 -topk8 -nocrypt -in "$work/basalt-test.key" -out basalt-test-key.pem
openssl pkcs8 -topk8 -nocrypt -in "$work/stranger-test.key" -out stranger-test-key.pem
echo "regenerate: fixtures written to $(pwd)"
