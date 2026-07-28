---
type: Attested Computation
parameters:
  - { name: year, type: integer, required: true }
---

# Computation

    SELECT SUM(amount) AS revenue
    FROM finance.recognized_revenue
    WHERE fiscal_year = @year
