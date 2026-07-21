# Security Policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 1.0.x   | yes       |
| 0.6.x   | security fixes only |
| < 0.6   | no        |

## Reporting

NoiseScope parses local files only - transcripts, protocol models and fixtures.
It performs no network I/O and implements no cryptography itself; it is an
analysis instrument, not a TLS implementation. If you find a vulnerability
(panics on crafted input, path traversal in file loading, unsafe deserialization),
open a private security advisory rather than a public issue. Expect a first
response within 7 days.

# draft note 75
