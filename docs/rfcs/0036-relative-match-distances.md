# RFC 0036: Predict match distance from the previous match

- Status: Rejected at Calgary opportunity gate
- Date: 2026-11-25
- Scope: Classical probability diagnostic

## 1. Hypothesis

Successive LZ matches can copy from the same earlier region even when their
distances differ. A signed difference from the previous distance may cost
less than an absolute distance. RFC 0016's follow-up counted exact repeats
and omitted selector cost. This experiment also tests nonzero differences
and accounts for selecting the predictor.

## 2. Control and candidate

Use the frozen RFC 0034 parse. Score every match distance. The control uses
TK32's adaptive distribution over 32 base-2 buckets for absolute distances
and one bit per bucket remainder bit.

In the alternate predictor, score the first distance under the control.
For later distances compute the signed difference from the previous match
distance, even across literal blocks. Map zero to 1, a positive difference
of magnitude m to 2m, and a negative difference of magnitude m to 2m+1.
Use a separate 32-symbol bucket distribution with the same initialization,
updates, and rescaling. Each file starts with fresh state.

Use an equal-prior Bayesian mixture of both predictors across each file.
Compute its ideal loss from the two summed losses, as in RFC 0035.
This charges model selection without an oracle or per-match switches.
Count exact repeats and nonzero differences separately in the raw results.
Keep all other syntax and probabilities outside the diagnostic.

## 3. Gate

Advance if the mixture saves at least 0.010 ideal bits per complete input
byte across Calgary and improves at least 12 of 18 files. Then apply the
frozen predictor to Canterbury, excluding ptt5 because it duplicates
Calgary pic. Require aggregate savings of at least 0.006 ideal bits per
input byte and improvement on a majority of those files.

Passing both screens justifies a separate complete-artifact experiment.
That experiment must verify exact restoration, account for every stream
byte, and measure encoder and decoder resources before full enwik9
validation. A failing screen ends this experiment before codec work.

## 4. Result

The mixture saves 3,506.746 ideal bits in aggregate, or 0.001079 bits
per input byte. It improves geo, pic, and progp. Both Calgary gates
fail. The standalone signed predictor spends 64,270.091 additional
ideal bits on nonzero differences and saves 14,223.681 on exact repeats.
Canterbury, complete artifacts, and enwik9 remain unmeasured for this
candidate.

[Experiment report](../../research/experiments/2026-11-25-relative-match-distances.md)
