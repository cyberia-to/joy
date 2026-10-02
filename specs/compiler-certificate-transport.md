# Disclosed compiler certificate transport

Status: transport component contract. Semantic record admission, production
dispatch and complete compiler proof acceptance are separate dependent work.
This component carries public witness bytes and asserts transport integrity.
Zheng owns the checked evaluation relation.

## Stream

All integers are unsigned little endian. The preamble is eight bytes
`JOYSC001`, followed by a 32-byte context supplied independently by the caller.
The reader compares that context before reading a frame. The context identifies
the admitted machine/profile, complete ART1, extracted formula, input noun,
initial reduction budget and evaluator frame allowance. Its application encoding
belongs to the production certificate contract.

Each frame has a 50-byte header, encoded payload and 32-byte Hemera digest:

| Offset | Bytes | Meaning |
|---|---:|---|
| 0 | 8 | Sequence, starting at zero |
| 8 | 1 | Kind: data0 or terminal1 |
| 9 | 1 | Codec: raw0 or raw DEFLATE1 |
| 10 | 4 | Decoded byte count |
| 14 | 4 | Encoded byte count |
| 18 | 32 | Previous digest |

The initial previous digest is Hemera of
`joy/compiler-certificate/context/v1\0 || preamble`. Each frame digest is Hemera
of `joy/compiler-certificate/frame/v1\0 || header || encoded payload`.
Sequence and previous digest must match the reader's checked state. Counts and
total allowances are checked before payload allocation, hashing or decoding.
Both chains cover the supplied context. Semantic admission independently binds
all public roots; a self-consistent transport chain alone proves no computation.

Data frames contain 1..65536 decoded bytes. Raw frames have equal encoded and
decoded counts. DEFLATE frames have a positive encoded count strictly smaller
than their decoded count. Decoding must finish, consume every encoded byte and
produce exactly the declared count. The bounded output buffer has one extra
byte to detect excess output. Valid DEFLATE bit encodings may differ; frame
digests cover the exact encoded bytes, including final bit padding. The writer
uses compression level1 only when it makes a frame smaller.

Terminal frames have codec0 and both counts zero. They close the frame chain.
The reader requires immediate underlying EOF and rejects extra bytes, even a
second valid frame. Absence of a complete terminal frame rejects. The semantic
parser must consume its own final record and call `finish` on the transport;
transport success alone supplies no semantic success. Reading a verified prefix
never constitutes acceptance of the certificate.

## Bounds and errors

Callers supply positive caps on total encoded wire bytes, decoded payload bytes
and frame count. Wire accounting includes the preamble, headers and digests;
frame accounting includes the terminal. No complete-stream buffer is retained.
Each frame uses at most 65536 encoded bytes and 65537 decoded bytes plus fixed
codec state. The writer reserves two 65536-byte payload buffers fallibly and
reuses them, abandoning compression when it cannot reduce a frame. Reader and
writer payload reservation failures return `OutOfMemory` without allocating an
error message. The pinned miniz compressor constructor internally allocates two
fixed-size private workspaces using Rust's infallible allocator; exhaustion at
that producer-only initialization follows the global allocation failure handler.
Those workspaces are reused and have no input-dependent growth. The verifier
uses fixed stack codec state and fallible payload buffers. Decoded totals use checked u64
arithmetic and sequence increment must not overflow.

Reader and writer become unusable after a framing, I/O, allocation or bound
error. EOF is returned only after a complete verified terminal. Empty read/write
requests follow the standard I/O contract. Writer `finish` consumes the writer,
flushes pending data, emits the terminal and flushes the destination. Interrupted
or failed destinations may contain a prefix; the production owner must stage
such bytes privately and publish only after full semantic and I/O success.

This layer has no clock or cancellation policy. Production wraps its bounded
operations in deadline checks and bounds semantic work separately. A generic
caller controls its I/O source, whose blocking behavior remains outside this
byte codec's contract.
