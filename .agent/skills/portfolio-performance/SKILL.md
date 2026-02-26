---
name: portfolio-performance
description: Use when implementing or changing portfolio calculations (PnL, returns, cost basis, performance periods: 1D/1M/1Y/YTD). Enforce consistent definitions and edge-case handling.
version: 1.0.0
scope: workspace
tags: [finance, pnl, portfolio, performance]
---

# Portfolio Performance Calculations

## Goal
Ensure portfolio metrics are correct, auditable, and consistent across UI/API.

## When to use
- Adding/changing PnL, returns, allocation, cost basis
- Implementing period metrics (1D/1M/1Y/YTD)
- Handling deposits/withdrawals/fees/dividends/corporate actions

## Definitions (baseline)
- **Position**: holdings per symbol (quantity, avg_cost, realized_pnl, unrealized_pnl)
- **Realized PnL**: from closed lots/trades
- **Unrealized PnL**: `(last_price - avg_cost) * quantity`
- **Total PnL**: realized + unrealized - fees (if tracked)
- **Return**: must specify method (simple vs TWR vs MWR)

## Rules
1. Choose cost basis method explicitly:
   - FIFO (default) OR Average Cost (document)
2. Decide how to treat:
   - fees (add to cost / subtract from pnl)
   - dividends (cashflow vs reinvest)
   - corporate actions (split/bonus) — if not supported, mark as limitation
3. Period performance must state:
   - price source (close price)
   - calendar vs trading days
   - timezone and “as-of” cutoff

## Required edge cases
- Partial fills / multiple buys
- Sell > holdings (reject or short? choose and enforce)
- Missing price on a date (fallback strategy)
- Cash deposits/withdrawals affect MWR; do not mix with simple return silently

## Implementation steps
1. Confirm requirements:
   - cost basis: FIFO or Avg
   - include fees/dividends? (if unknown, default: fees included, dividends separate)
2. Implement calculation in a pure function/module.
3. Add unit tests with known scenarios:
   - buy->buy->sell partial
   - buy->sell all
   - price gap / missing day
4. Expose via API with transparent fields:
   - inputs used (as-of date/time, price source)
   - results (pnl/return breakdown)
5. Ensure UI labels match the definition.

## Output expectations
- Provide formulas used
- Provide example scenario with computed numbers
- Provide tests or test vectors

## Quick checklist
- [ ] Cost basis method explicit
- [ ] Fees/dividends handling explicit
- [ ] Period definition explicit (cutoff/timezone)
- [ ] Tests cover partial sell + missing price
- [ ] API output includes breakdown fields
