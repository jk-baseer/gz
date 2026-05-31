use rand::Rng;
use rand_distr::{Beta, Distribution};
use uuid::Uuid;

/// Runs Monte Carlo Thompson Sampling and returns the allocation probability
/// (fraction of simulated auctions won) for each creative.
///
/// Input: slice of (creative_id, total_impressions, total_clicks)
/// Output: same order, with probability in [0.0, 1.0] summing to ~1.0
pub fn compute_allocation_probabilities(
    creatives: &[(Uuid, i64, i64)],
) -> Vec<(Uuid, f64)> {
    if creatives.is_empty() {
        return vec![];
    }
    // Single creative always gets 100% — skip simulation
    if creatives.len() == 1 {
        return vec![(creatives[0].0, 1.0)];
    }

    const N_SIMS: u32 = 10_000;
    let mut wins = vec![0u32; creatives.len()];
    let mut rng = rand::thread_rng();

    // Build Beta distributions once per creative (resampled each simulation)
    let params: Vec<(f64, f64)> = creatives
        .iter()
        .map(|(_, impressions, clicks)| {
            let alpha = (*clicks + 1) as f64;
            let beta = ((impressions - clicks).max(0) + 1) as f64;
            (alpha, beta)
        })
        .collect();

    for _ in 0..N_SIMS {
        let mut best_score = -1.0f64;
        let mut best_idx = 0usize;

        for (i, &(alpha, beta)) in params.iter().enumerate() {
            let score = match Beta::new(alpha, beta) {
                Ok(dist) => dist.sample(&mut rng),
                Err(_) => rng.gen::<f64>(),
            };
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }
        wins[best_idx] += 1;
    }

    creatives
        .iter()
        .enumerate()
        .map(|(i, (id, _, _))| (*id, wins[i] as f64 / N_SIMS as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_creative_gets_full_allocation() {
        let id = Uuid::new_v4();
        let result = compute_allocation_probabilities(&[(id, 100, 10)]);
        assert_eq!(result.len(), 1);
        assert!((result[0].1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn high_ctr_creative_dominates() {
        let id_good = Uuid::new_v4();
        let id_bad = Uuid::new_v4();
        // good: 50% CTR (50/100), bad: 2% CTR (2/100)
        let result = compute_allocation_probabilities(&[
            (id_good, 100, 50),
            (id_bad,  100,  2),
        ]);
        let prob_good = result.iter().find(|(id, _)| *id == id_good).unwrap().1;
        let prob_bad  = result.iter().find(|(id, _)| *id == id_bad).unwrap().1;
        // Good creative should win the vast majority of simulations
        assert!(prob_good > 0.90, "expected good creative >90% allocation, got {prob_good:.3}");
        assert!(prob_bad < 0.10,  "expected bad creative <10% allocation, got {prob_bad:.3}");
    }

    #[test]
    fn new_creatives_share_allocation_equally() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        // All creatives have zero impressions
        let input: Vec<(Uuid, i64, i64)> = ids.iter().map(|id| (*id, 0, 0)).collect();
        let result = compute_allocation_probabilities(&input);
        for (_, prob) in &result {
            // Each should get roughly 1/3 (allow ±10% for Monte Carlo variance)
            assert!(*prob > 0.23 && *prob < 0.44, "unequal allocation on zero data: {prob:.3}");
        }
    }
}
