"""Ordinary-workload regression budget, in exact bytes of Private_Dirty.

The original 3 MB target remains visible. On 2026-09-09 the user prioritized
CPU, responsiveness and correctness over forcing that target. Three normal
allocator runs peaked at 3.23–3.30 MB; 4 MB bounds regressions with headroom.
This does not replace latency, CPU, swap or external-resource verification.
"""
TARGET_BYTES = 3_000_000
LIMIT_BYTES = 4_000_000
