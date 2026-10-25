---
id: feat.finance
title: Finance & Invoices
category: features
domain: finance
status: implemented
source_paths: [crates/aevio-services/src/finance.rs, crates/aevio-core/src/invoices/, crates/aevio-core/src/clients/, apps/web/src/features/finance/, apps/web/src/features/invoices/, apps/web/src/features/clients/]
related_docs: [docs/features/time-tracking.md]
platforms: [macos, windows]
verification_commit: working-tree
---

# Finance & Invoices

Invoices, clients, finance records. Money is integer USD cents in Rust. Invoice PDFs composed in Rust (printpdf), displayed via pdf.js. Never compute totals in TS.
