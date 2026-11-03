# 0001 — Keep the Sarv name and visual identity

**Status:** accepted · **Date:** 2026-11-03

## Context
The spec is written for a product called "Daftar" and says "rename freely". It also prescribes a
primary colour of `oklch(0.55 0.12 230)` and Vazirmatn as the one typeface. Sarv already ships a
teal accent, a sprout mark and an IBM Plex Sans / Space Grotesk / Vazirmatn type system. The owner has
said that redesigns must keep that identity and that "Apple-like" means restraint, not Apple visuals.

## Decision
The product stays **Sarv**. The spec's design tokens are treated as guidance for *structure*
(semantic status colours, debit/credit never red/green, tabular numerals, density, logical CSS
properties), not as a replacement palette or font. Vazirmatn is already bundled for Persian text.

## Consequences
Spec items that name Daftar (`daftar://` deep links, `daftar verify-ledger`, `company.daftar`) map to
Sarv equivalents when they are built.
