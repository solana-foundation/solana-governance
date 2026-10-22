/// Return a sample count bounding the chance of missing any uniformly selected verifier.
pub fn required_samples(verifiers: usize, confidence: f64) -> usize {
    if verifiers <= 1 {
        return verifiers;
    }
    let count = verifiers as f64;
    // The union bound gives P(any missed) <= N * (1 - 1/N)^samples.
    (((1.0 - confidence) / count).ln() / (-1.0 / count).ln_1p()).ceil() as usize
}

pub fn miss_probability_bound(verifiers: usize, samples: usize) -> f64 {
    if samples == 0 {
        return 1.0;
    }
    if verifiers <= 1 {
        return 0.0;
    }
    let count = verifiers as f64;
    (count * ((-1.0 / count).ln_1p() * samples as f64).exp()).min(1.0)
}
