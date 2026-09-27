# Deterministic codec dependency

Joy source `c2aa7862f46ea3f70f9a694c9c33d92e383ced5c` selects the existing0.7.1 macro through the checksum-
pinned Trisha overlay at `765819f02b2dd69b83dd083544072d07352a5237`. The lockfile changes only the source
of `bfieldcodec_derive`; its version and all other dependency records stay fixed.

The coordinated [Trisha receipt](../../trisha/audit/deterministic-codec-validation.json)
records1175/122/380 owner tests,133 unchanged benchmark rows, the reproduced
upstream regression and two fresh byte-identical builds of all three binaries.
Source enum encoding remains unchanged. This fixes build determinism at the same
source and target paths; compiler self-hosting and native proofs remain separate.
