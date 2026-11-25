# RFC 0035: Bit-group literal prediction

- Status: Rejected at Calgary opportunity gate
- Date: 2026-11-25
- Scope: Classical probability diagnostic

The largest remaining Calgary deficits to xz are on obj2, geo, and pic.
Test whether splitting literal bytes into two four-bit groups helps
when the groups have dependencies obscured by whole-byte contexts.

Use the frozen RFC 0034 parse. The control scores its literals with
the complete TK32 model. The alternate model predicts the upper group
from the previous two upper groups. It predicts the lower group from
the previous two lower groups and the already-known current upper group.
Initialize previous bytes to zero. Each 16-symbol row starts with one
count per symbol. Before updating a row at total 4,096, halve counts
rounding up. Both models observe every source byte, including match copies.

Use an equal-prior Bayesian mixture of the two literal predictors.
After each scored event, update expert weights by their likelihoods.
For each whole file, compute the equivalent mixture loss as
`-log2((2^-L_tk32 + 2^-L_groups)/2)` in log space. This charges
the model-selection cost and uses no oracle choice per literal.
Separate the two expert losses and the mixture loss in raw results.
The diagnostic uses floating point. A later implementation would
require deterministic integer frequencies and complete artifact accounting.

Run Calgary first. Advance only if the mixture saves at least 0.03
ideal bits per input byte overall and at least 0.02 across obj2,
geo, and pic combined. Then validate on independent Canterbury files
with the same settings and at least 0.02 bits saved per input byte
overall. Exclude ptt5 because it duplicates Calgary pic. A passing
screen authorizes a separate artifact experiment before enwik9.

## Result

The mixture saves 68,123.064 ideal bits overall, or 0.020951 per
input byte. This falls below the 0.03 requirement. The three-file
subset saves 0.079007 bits per input byte, driven by geo.
The experiment stops before Canterbury and artifact implementation.
[Experiment report](../../research/experiments/2026-11-25-nibble-literal-mixture.md)
