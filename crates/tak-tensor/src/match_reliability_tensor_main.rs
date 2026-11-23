use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match-reliability-tensor: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = args
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or("expected INPUT --rank R [--tail-bytes N] [--epochs N] [--learning-rate X]")?;
    let rank = option(&args, "--rank")
        .ok_or("--rank is required")?
        .parse::<usize>()
        .map_err(|_| "rank must be 1, 2, 4, or 8")?;
    if ![1, 2, 4, 8].contains(&rank) {
        return Err("rank must be 1, 2, 4, or 8".to_owned());
    }
    let epochs = option(&args, "--epochs")
        .unwrap_or("10")
        .parse::<usize>()
        .map_err(|_| "epochs must be a positive integer")?;
    if epochs == 0 {
        return Err("epochs must be a positive integer".to_owned());
    }
    let learning_rate = option(&args, "--learning-rate")
        .unwrap_or("0.05")
        .parse::<f64>()
        .map_err(|_| "learning-rate must be a positive finite number")?;
    if !learning_rate.is_finite() || learning_rate <= 0.0 {
        return Err("learning-rate must be a positive finite number".to_owned());
    }
    let tail_bytes = option(&args, "--tail-bytes")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "tail-bytes must be a nonnegative integer")
        })
        .transpose()?;

    let input = fs::read(PathBuf::from(path)).map_err(|error| error.to_string())?;
    let input = if let Some(bytes) = tail_bytes {
        let start = input
            .len()
            .checked_sub(bytes)
            .ok_or("tail-bytes exceeds input length")?;
        &input[start..]
    } else {
        &input
    };
    if input.len() < 20 {
        return Err("input is too short for a 90/10 match-event split".to_owned());
    }
    let split = input.len() * 9 / 10;
    let training = collect_events(&input[..split], false);
    let evaluation = collect_events(&input[split..], true);
    if training.events.is_empty() || evaluation.events.is_empty() {
        return Err("both partitions must contain exact-suffix match events".to_owned());
    }

    let start = Instant::now();
    let model = Model::train(&training.events, rank, epochs, learning_rate);
    let training_seconds = start.elapsed().as_secs_f64();
    let training_bpe = model.bits_per_event(&training.events);
    let float_bpe = model.bits_per_event(&evaluation.events);
    let quantized = QuantizedModel::from_float(&model);
    let quantized_bpe = quantized.float_bits_per_event(&evaluation.events);
    let integer_bpe = quantized.integer_bits_per_event(&evaluation.events);
    let model_bits =
        u32::try_from(quantized.model_bytes() * 8).expect("bounded tensor metadata fits u32");
    let metadata_bpe = divide(f64::from(model_bits), evaluation.events.len());

    println!(
        "rank\tinput_bytes\ttrain_bytes\teval_bytes\ttrain_events\teval_events\tepochs\tmodel_bytes\tglobal_bpe\tfeature_bpe\tsuffix_bpe\ttraining_bpe\tfloat_bpe\tquantized_bpe\tinteger_bpe\tmetadata_bpe\tinteger_with_metadata_bpe\ttraining_seconds"
    );
    println!(
        "{rank}\t{}\t{split}\t{}\t{}\t{}\t{epochs}\t{}\t{:.9}\t{:.9}\t{:.9}\t{training_bpe:.9}\t{float_bpe:.9}\t{quantized_bpe:.9}\t{integer_bpe:.9}\t{metadata_bpe:.9}\t{:.9}\t{training_seconds:.9}",
        input.len(),
        input.len() - split,
        training.events.len(),
        evaluation.events.len(),
        quantized.model_bytes(),
        divide(evaluation.global_bits, evaluation.events.len()),
        divide(evaluation.feature_bits, evaluation.events.len()),
        divide(evaluation.suffix_bits, evaluation.events.len()),
        integer_bpe + metadata_bpe,
    );
    Ok(())
}

fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

#[derive(Clone, Copy, Default)]
struct MatchEntry {
    key: u64,
    continuation: u8,
    position: u32,
    occupied: bool,
    reliability: Counts,
}

#[derive(Clone, Copy)]
struct Counts {
    correct: u32,
    incorrect: u32,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            correct: 1,
            incorrect: 1,
        }
    }
}

impl Counts {
    fn bits(self, correct: bool) -> f64 {
        let total = self.correct + self.incorrect;
        let count = if correct {
            self.correct
        } else {
            self.incorrect
        };
        f64::from(total).log2() - f64::from(count).log2()
    }

    fn update(&mut self, correct: bool) {
        if correct {
            self.correct += 1;
        } else {
            self.incorrect += 1;
        }
    }
}

#[derive(Clone)]
struct Event {
    distance: usize,
    run: usize,
    suffix: [u8; 8],
    correct: bool,
}

struct EventSet {
    events: Vec<Event>,
    global_bits: f64,
    feature_bits: f64,
    suffix_bits: f64,
}

fn collect_events(input: &[u8], score_controls: bool) -> EventSet {
    let mut table = vec![MatchEntry::default(); TABLE_LEN];
    let mut global = Counts::default();
    let mut features = [[Counts::default(); 6]; 4];
    let mut history = 0_u64;
    let mut seen = 0_u8;
    let mut correct_run = 0_u32;
    let mut result = EventSet {
        events: Vec::new(),
        global_bits: 0.0,
        feature_bits: 0.0,
        suffix_bits: 0.0,
    };
    for (position, &actual) in input.iter().enumerate() {
        let position = u32::try_from(position).expect("partition length is bounded to u32");
        if seen >= 8 {
            let index = table_index(history);
            let mut entry = table[index];
            if entry.occupied && entry.key == history {
                let correct = entry.continuation == actual;
                let distance = distance_bucket(position - entry.position);
                let run = run_bucket(correct_run);
                if score_controls {
                    result.global_bits += global.bits(correct);
                    result.feature_bits += features[distance][run].bits(correct);
                    result.suffix_bits += entry.reliability.bits(correct);
                    global.update(correct);
                    features[distance][run].update(correct);
                    entry.reliability.update(correct);
                }
                result.events.push(Event {
                    distance,
                    run,
                    suffix: history.to_be_bytes(),
                    correct,
                });
                correct_run = if correct { correct_run + 1 } else { 0 };
            } else {
                correct_run = 0;
            }
            if entry.occupied && entry.key == history {
                entry.continuation = actual;
                entry.position = position;
                table[index] = entry;
            } else {
                table[index] = MatchEntry {
                    key: history,
                    continuation: actual,
                    position,
                    occupied: true,
                    reliability: Counts::default(),
                };
            }
        }
        history = (history << 8) | u64::from(actual);
        seen = seen.saturating_add(1).min(8);
    }
    result
}

fn distance_bucket(distance: u32) -> usize {
    match distance {
        0..=255 => 0,
        256..=4_095 => 1,
        4_096..=65_535 => 2,
        _ => 3,
    }
}

fn run_bucket(run: u32) -> usize {
    match run {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=7 => 3,
        8..=15 => 4,
        _ => 5,
    }
}

fn table_index(key: u64) -> usize {
    let value = key.wrapping_mul(HASH_MULTIPLIER) >> (64 - TABLE_BITS);
    usize::try_from(value).expect("20-bit index fits usize")
}

struct Model {
    rank: usize,
    distance: Vec<f64>,
    run: Vec<f64>,
    byte: Vec<f64>,
    outcome: Vec<f64>,
}

impl Model {
    fn new(rank: usize) -> Self {
        let distance = seeded_values(4 * rank, 0);
        let run = near_identity_cores(6, rank, 4 * rank);
        let byte = near_identity_cores(256, rank, 4 * rank + 6 * rank * rank);
        let outcome = if rank == 1 {
            vec![1.0, 1.0]
        } else {
            (0..rank)
                .flat_map(|hidden| {
                    let position =
                        f64::from(u32::try_from(hidden).expect("rank is bounded to eight"))
                            / f64::from(u32::try_from(rank - 1).expect("rank exceeds one"));
                    [1.5 - position, 0.5 + position]
                })
                .collect()
        };
        Self {
            rank,
            distance,
            run,
            byte,
            outcome,
        }
    }

    fn train(events: &[Event], rank: usize, epochs: usize, learning_rate: f64) -> Self {
        let mut model = Self::new(rank);
        for epoch in 0..epochs {
            let mut gradient = Gradient::new(rank);
            for event in events {
                model.accumulate_gradient(event, &mut gradient);
            }
            let event_count =
                f64::from(u32::try_from(events.len()).expect("a diagnostic partition fits u32"));
            let epoch_number = f64::from(u32::try_from(epoch + 1).expect("epoch count fits u32"));
            let rate = learning_rate / (event_count * epoch_number.sqrt());
            update(&mut model.distance, &gradient.distance, rate);
            update(&mut model.run, &gradient.run, rate);
            update(&mut model.byte, &gradient.byte, rate);
            update(&mut model.outcome, &gradient.outcome, rate);
        }
        model
    }

    fn accumulate_gradient(&self, event: &Event, gradient: &mut Gradient) {
        let rank = self.rank;
        let distance_start = event.distance * rank;
        let mut states = Vec::with_capacity(10);
        let mut sums = Vec::with_capacity(10);
        let (state, sum) = normalize(&self.distance[distance_start..distance_start + rank]);
        states.push(state);
        sums.push(sum);

        let modes = std::iter::once((event.run, &self.run[..])).chain(
            event
                .suffix
                .iter()
                .map(|&byte| (usize::from(byte), &self.byte[..])),
        );
        for (step, (mode, core)) in modes.enumerate() {
            let dimensions = if step == 0 { 6 } else { 256 };
            let (next, sum) = transition(
                states.last().expect("distance state exists"),
                core,
                mode,
                dimensions,
                rank,
            );
            states.push(next);
            sums.push(sum);
        }

        let final_state = states.last().expect("final state exists");
        let mut scores = [0.0; 2];
        for (outcome, score) in scores.iter_mut().enumerate() {
            *score = (0..rank)
                .map(|hidden| final_state[hidden] * self.outcome[hidden * 2 + outcome])
                .sum();
        }
        let actual = usize::from(event.correct);
        let score_sum = scores[0] + scores[1];
        let mut score_gradient = [1.0 / score_sum; 2];
        score_gradient[actual] -= 1.0 / scores[actual];
        let mut state_gradient = vec![0.0; rank];
        for hidden in 0..rank {
            for (outcome, &score_derivative) in score_gradient.iter().enumerate() {
                gradient.outcome[hidden * 2 + outcome] += final_state[hidden] * score_derivative;
                state_gradient[hidden] += self.outcome[hidden * 2 + outcome] * score_derivative;
            }
        }

        for step in (0..9).rev() {
            let (mode, core, core_gradient, dimensions) = if step == 0 {
                (event.run, &self.run, &mut gradient.run, 6)
            } else {
                (
                    usize::from(event.suffix[step - 1]),
                    &self.byte,
                    &mut gradient.byte,
                    256,
                )
            };
            state_gradient = transition_backward(
                &states[step],
                &states[step + 1],
                sums[step + 1],
                &state_gradient,
                core,
                core_gradient,
                mode,
                dimensions,
                rank,
            );
        }
        let dot: f64 = state_gradient
            .iter()
            .zip(&states[0])
            .map(|(gradient, state)| gradient * state)
            .sum();
        for (hidden, &derivative) in state_gradient.iter().enumerate() {
            gradient.distance[distance_start + hidden] += (derivative - dot) / sums[0];
        }
    }

    fn bits_per_event(&self, events: &[Event]) -> f64 {
        divide(
            events.iter().map(|event| self.event_bits(event)).sum(),
            events.len(),
        )
    }

    fn event_bits(&self, event: &Event) -> f64 {
        let rank = self.rank;
        let start = event.distance * rank;
        let (mut state, _) = normalize(&self.distance[start..start + rank]);
        state = transition(&state, &self.run, event.run, 6, rank).0;
        for &byte in &event.suffix {
            state = transition(&state, &self.byte, usize::from(byte), 256, rank).0;
        }
        let scores = outcome_scores(&state, &self.outcome, rank);
        probability_bits(scores, event.correct)
    }
}

fn seeded_values(length: usize, offset: usize) -> Vec<f64> {
    (0..length)
        .map(|index| {
            let seed = (offset + index).wrapping_mul(37).wrapping_add(11) % 29;
            0.5 + f64::from(u32::try_from(seed).expect("seed is less than 29")) / 28.0
        })
        .collect()
}

fn near_identity_cores(modes: usize, rank: usize, offset: usize) -> Vec<f64> {
    let seeds = seeded_values(modes * rank * rank, offset);
    let mut values = vec![0.0; seeds.len()];
    for mode in 0..modes {
        for left in 0..rank {
            for right in 0..rank {
                let index = (mode * rank + left) * rank + right;
                values[index] = if left == right {
                    0.95 + seeds[index] * 0.1
                } else {
                    0.005 + seeds[index] * 0.01
                };
            }
        }
    }
    values
}

struct Gradient {
    distance: Vec<f64>,
    run: Vec<f64>,
    byte: Vec<f64>,
    outcome: Vec<f64>,
}

impl Gradient {
    fn new(rank: usize) -> Self {
        Self {
            distance: vec![0.0; 4 * rank],
            run: vec![0.0; 6 * rank * rank],
            byte: vec![0.0; 256 * rank * rank],
            outcome: vec![0.0; 2 * rank],
        }
    }
}

fn normalize(values: &[f64]) -> (Vec<f64>, f64) {
    let sum: f64 = values.iter().sum();
    (values.iter().map(|value| value / sum).collect(), sum)
}

fn transition(
    state: &[f64],
    core: &[f64],
    mode: usize,
    dimensions: usize,
    rank: usize,
) -> (Vec<f64>, f64) {
    debug_assert!(mode < dimensions);
    let start = mode * rank * rank;
    let mut next = vec![0.0; rank];
    for right in 0..rank {
        next[right] = (0..rank)
            .map(|left| state[left] * core[start + left * rank + right])
            .sum();
    }
    normalize(&next)
}

#[allow(clippy::too_many_arguments)]
fn transition_backward(
    previous: &[f64],
    current: &[f64],
    sum: f64,
    current_gradient: &[f64],
    core: &[f64],
    core_gradient: &mut [f64],
    mode: usize,
    dimensions: usize,
    rank: usize,
) -> Vec<f64> {
    debug_assert!(mode < dimensions);
    let start = mode * rank * rank;
    let dot: f64 = current_gradient
        .iter()
        .zip(current)
        .map(|(gradient, state)| gradient * state)
        .sum();
    let normalized_gradient: Vec<f64> = current_gradient
        .iter()
        .map(|gradient| (gradient - dot) / sum)
        .collect();
    let mut previous_gradient = vec![0.0; rank];
    for left in 0..rank {
        for (right, &derivative) in normalized_gradient.iter().enumerate() {
            let index = start + left * rank + right;
            core_gradient[index] += previous[left] * derivative;
            previous_gradient[left] += core[index] * derivative;
        }
    }
    previous_gradient
}

fn outcome_scores(state: &[f64], outcome: &[f64], rank: usize) -> [f64; 2] {
    let mut scores = [0.0; 2];
    for hidden in 0..rank {
        for result in 0..2 {
            scores[result] += state[hidden] * outcome[hidden * 2 + result];
        }
    }
    scores
}

fn probability_bits(scores: [f64; 2], correct: bool) -> f64 {
    let actual = usize::from(correct);
    (scores[0] + scores[1]).log2() - scores[actual].log2()
}

fn update(values: &mut [f64], gradient: &[f64], rate: f64) {
    for (value, &gradient) in values.iter_mut().zip(gradient) {
        let log_step = (-rate * gradient * *value).clamp(-0.25, 0.25);
        *value = (*value * log_step.exp()).clamp(1.0e-4, 1.0e4);
    }
}

struct QuantizedModel {
    rank: usize,
    distance: Vec<u16>,
    run: Vec<u16>,
    byte: Vec<u16>,
    outcome: Vec<u16>,
}

impl QuantizedModel {
    fn from_float(model: &Model) -> Self {
        Self {
            rank: model.rank,
            distance: quantize(&model.distance),
            run: quantize(&model.run),
            byte: quantize(&model.byte),
            outcome: quantize(&model.outcome),
        }
    }

    fn model_bytes(&self) -> usize {
        (self.distance.len() + self.run.len() + self.byte.len() + self.outcome.len()) * 2
    }

    fn float_bits_per_event(&self, events: &[Event]) -> f64 {
        divide(
            events
                .iter()
                .map(|event| self.float_event_bits(event))
                .sum(),
            events.len(),
        )
    }

    fn float_event_bits(&self, event: &Event) -> f64 {
        let rank = self.rank;
        let start = event.distance * rank;
        let values: Vec<f64> = self.distance[start..start + rank]
            .iter()
            .map(|&value| f64::from(value))
            .collect();
        let (mut state, _) = normalize(&values);
        state = quantized_float_transition(&state, &self.run, event.run, rank);
        for &byte in &event.suffix {
            state = quantized_float_transition(&state, &self.byte, usize::from(byte), rank);
        }
        let outcome: Vec<f64> = self.outcome.iter().map(|&value| f64::from(value)).collect();
        probability_bits(outcome_scores(&state, &outcome, rank), event.correct)
    }

    fn integer_bits_per_event(&self, events: &[Event]) -> f64 {
        divide(
            events
                .iter()
                .map(|event| self.integer_event_bits(event))
                .sum(),
            events.len(),
        )
    }

    fn integer_event_bits(&self, event: &Event) -> f64 {
        let rank = self.rank;
        let start = event.distance * rank;
        let mut state = integer_normalize(
            &self.distance[start..start + rank]
                .iter()
                .map(|&value| u64::from(value))
                .collect::<Vec<_>>(),
        );
        state = integer_transition(&state, &self.run, event.run, rank);
        for &byte in &event.suffix {
            state = integer_transition(&state, &self.byte, usize::from(byte), rank);
        }
        let mut scores = [0_u64; 2];
        for (hidden, &hidden_state) in state.iter().enumerate() {
            for (outcome, score) in scores.iter_mut().enumerate() {
                *score += u64::from(hidden_state) * u64::from(self.outcome[hidden * 2 + outcome]);
            }
        }
        integer_probability_bits(scores, event.correct)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize(values: &[f64]) -> Vec<u16> {
    let maximum = values.iter().copied().fold(0.0_f64, f64::max);
    values
        .iter()
        .map(|value| {
            let scaled = 1.0 + value / maximum * 65_534.0;
            scaled.round().clamp(1.0, 65_535.0) as u16
        })
        .collect()
}

fn quantized_float_transition(state: &[f64], core: &[u16], mode: usize, rank: usize) -> Vec<f64> {
    let start = mode * rank * rank;
    let mut next = vec![0.0; rank];
    for right in 0..rank {
        next[right] = (0..rank)
            .map(|left| state[left] * f64::from(core[start + left * rank + right]))
            .sum();
    }
    normalize(&next).0
}

fn integer_transition(state: &[u16], core: &[u16], mode: usize, rank: usize) -> Vec<u16> {
    let start = mode * rank * rank;
    let mut next = vec![0_u64; rank];
    for right in 0..rank {
        next[right] = (0..rank)
            .map(|left| u64::from(state[left]) * u64::from(core[start + left * rank + right]))
            .sum();
    }
    integer_normalize(&next)
}

fn integer_normalize(values: &[u64]) -> Vec<u16> {
    let maximum = values.iter().copied().max().unwrap_or(0);
    assert!(maximum > 0, "positive cores produce a positive contraction");
    values
        .iter()
        .map(|&value| {
            let scaled = 1 + u128::from(value) * 65_534 / u128::from(maximum);
            u16::try_from(scaled).expect("normalization is bounded to u16")
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn integer_probability_bits(scores: [u64; 2], correct: bool) -> f64 {
    // Scores are below 2^36 and are exactly representable in f64.
    probability_bits([scores[0] as f64, scores[1] as f64], correct)
}

fn divide(value: f64, events: usize) -> f64 {
    if events == 0 {
        0.0
    } else {
        let events = u32::try_from(events).expect("a diagnostic partition fits u32");
        value / f64::from(events)
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, QuantizedModel, collect_events};

    #[test]
    fn every_rank_produces_finite_losses_and_expected_metadata() {
        let input = b"abcdefghXabcdefghXabcdefghY".repeat(100);
        let training = collect_events(&input[..input.len() * 9 / 10], false);
        let evaluation = collect_events(&input[input.len() * 9 / 10..], true);
        for (rank, bytes) in [(1, 536), (2, 2_120), (4, 8_432), (8, 33_632)] {
            let model = Model::train(&training.events, rank, 1, 0.05);
            let quantized = QuantizedModel::from_float(&model);
            assert_eq!(quantized.model_bytes(), bytes);
            assert!(model.bits_per_event(&evaluation.events).is_finite());
            assert!(
                quantized
                    .float_bits_per_event(&evaluation.events)
                    .is_finite()
            );
            assert!(
                quantized
                    .integer_bits_per_event(&evaluation.events)
                    .is_finite()
            );
        }
    }
}
