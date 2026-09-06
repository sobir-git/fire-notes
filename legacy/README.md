# Legacy reference

`fire-notes/` contains the original application, its custom UI experiments, and historical tests and documentation. It is excluded from the active Cargo workspace. The new packages do not import it.

The original code has confirmed data-loss bugs. Preserve it as a behavior and implementation reference, not as a storage implementation to copy into the new framework. See the root architecture review.

Existing user notes were not moved or modified. The studio's notes exercise currently keeps its text in memory.
