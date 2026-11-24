# RFC 0029: Literal-only probability counts

- Status: Rejected at Calgary opportunity gate
- Date: 2026-11-24
- Scope: Classical literal probability diagnostic
- Format change: None

RFC 0019 uses normalized order-2 counts learned from all restored bytes
to predict the subset emitted as literals. Match copies can reinforce
continuations that the parser rarely emits as literals. Test whether
training on literals improves the resulting predictions.

Use the frozen RFC 0019 parse and literal model as the control. Create a
second copy of the literal model. On a literal, score and update each
model with its own probabilities and RFC 0017 phase mixture. On a copied
byte, the candidate updates both byte histories and the exact-suffix
table, while skipping the order-1 parent and order-2 local count updates.
Keep the prior strength, phase bins, and mixture rules fixed. Use one
shared normalized order-2 model across phases.

For both models, sum the negative base-2 logarithm of each literal's
integer frequency divided by its total. Report savings per complete
input byte. The remaining coding decisions are identical by construction.
The candidate state derives from the parse and restored bytes.

Start with the five established final-10% Silesia development tails.
Require positive savings on at least three files and at least 0.008 ideal
bits per input byte in aggregate. If this passes, freeze the candidate
and validate on all 18 Calgary files independently. Calgary requires
positive savings on at least 12 files and at least 0.008 ideal bits per
input byte in aggregate. Stop at the first failed corpus gate. Any
passing result requires a later complete-artifact experiment with exact
restoration and measured decoder time and memory before enwik9 validation.

## Result

Silesia improved on five of five tails, saving 300,222.870 ideal bits
(0.031764 bits per input byte). Calgary improved on ten of eighteen files
and saved 26,866.742 ideal bits (0.008263 bits per input byte). The Calgary
aggregate passed the threshold. The per-file condition failed, so the
experiment ended before artifact implementation.

See [the experiment report](../../research/experiments/2026-11-24-rejected-literal-only-counts.md).
