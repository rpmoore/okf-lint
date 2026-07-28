---
type: Metric
title: Revenue
description: Recognized revenue for a fiscal year.
resource: https://console.cloud.google.com/bigquery?p=acme&d=sales&t=orders
tags: [finance, revenue]
status: stable
stale_after: 2026-12-31
generated: { by: reference_agent/gemini-2.5-pro, at: 2026-06-20T22:53:05Z }
verified:
  - { by: human:ahormati, at: 2026-06-25T09:00:00Z }
  - { by: process:finance-nightly, at: 2026-06-26T02:00:00Z }
sources:
  - id: rev-policy
    resource: https://wiki.acme/finance/revenue-recognition
    title: Revenue recognition policy
    author: team:finance-fpa
    usage_count: 5000
    last_modified: 2026-04-02
usage_window: { from: 2026-06-01, to: 2026-06-30 }
---

# Definition

Recognized revenue sums `amount` over rows booked to the fiscal
year.[^rev-policy]

[^rev-policy]: Revenue recognition policy
