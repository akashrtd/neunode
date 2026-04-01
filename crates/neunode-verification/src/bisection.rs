/// Result of a Verde-style bisection search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct BisectionResult {
    pub found: bool,
    #[ts(type = "number")]
    pub disagreeing_op_index: Option<u32>,
    #[ts(type = "number")]
    pub steps_taken: u32,
    #[ts(type = "number")]
    pub total_ops: u32,
    pub claimant_hash: String,
    pub challenger_hash: String,
}

/// Verde-style binary search solver for finding first disagreeing op.
pub struct BisectionSolver;

impl BisectionSolver {
    pub fn new() -> Self {
        Self
    }

    /// Binary search between claimant and challenger hashes.
    ///
    /// `verify_fn(index)` returns true if hashes match at that index.
    pub fn solve<F>(
        &self,
        claimant_hashes: &[String],
        challenger_hashes: &[String],
        verify_fn: F,
    ) -> BisectionResult
    where
        F: Fn(u32) -> bool,
    {
        let empty = BisectionResult {
            found: false,
            disagreeing_op_index: None,
            steps_taken: 0,
            total_ops: 0,
            claimant_hash: String::new(),
            challenger_hash: String::new(),
        };

        if claimant_hashes.len() != challenger_hashes.len() {
            return empty;
        }
        if claimant_hashes.is_empty() {
            return empty;
        }

        let n = claimant_hashes.len() as u32;
        let empty_with_n = BisectionResult { total_ops: n, ..empty };

        if claimant_hashes == challenger_hashes {
            return empty_with_n;
        }

        let mut lo: u32 = 0;
        let mut hi: u32 = n - 1;
        let mut steps: u32 = 0;

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            steps += 1;
            if verify_fn(mid) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        BisectionResult {
            found: true,
            disagreeing_op_index: Some(lo),
            steps_taken: steps,
            total_ops: n,
            claimant_hash: claimant_hashes[lo as usize].clone(),
            challenger_hash: challenger_hashes[lo as usize].clone(),
        }
    }

    /// Linear scan for the first mismatching hash.
    pub fn verify_range(&self, hashes_a: &[String], hashes_b: &[String]) -> Option<u32> {
        if hashes_a.len() != hashes_b.len() {
            return None;
        }
        for (i, (a, b)) in hashes_a.iter().zip(hashes_b.iter()).enumerate() {
            if a != b {
                return Some(i as u32);
            }
        }
        None
    }
}

impl Default for BisectionSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_solver() {
        let _solver = BisectionSolver::new();
    }

    fn make_hashes(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("hash_{i}")).collect()
    }

    /// Split-point mismatch: all hashes before `split` match,
    /// all from `split` onward differ. This models the real Verde
    /// scenario where two computations diverge at a single point.
    fn make_split(n: usize, split: usize) -> (Vec<String>, Vec<String>) {
        let a: Vec<String> = (0..n).map(|i| format!("hash_{i}")).collect();
        let b: Vec<String> = (0..n)
            .map(|i| if i >= split { format!("DIFF_hash_{i}") } else { format!("hash_{i}") })
            .collect();
        (a, b)
    }

    #[test]
    fn agree_on_all_ops() {
        let solver = BisectionSolver::new();
        let hashes = make_hashes(10);
        let result =
            solver.solve(&hashes, &hashes, |idx| hashes[idx as usize] == hashes[idx as usize]);
        assert!(!result.found);
        assert!(result.disagreeing_op_index.is_none());
        assert_eq!(result.steps_taken, 0);
        assert_eq!(result.total_ops, 10);
    }

    #[test]
    fn disagree_on_first_op() {
        let solver = BisectionSolver::new();
        let (a, b) = make_split(10, 0);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        assert_eq!(result.disagreeing_op_index, Some(0));
        assert_eq!(result.total_ops, 10);
    }

    #[test]
    fn disagree_on_last_op() {
        let solver = BisectionSolver::new();
        let (a, b) = make_split(10, 9);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        assert_eq!(result.disagreeing_op_index, Some(9));
    }

    #[test]
    fn disagree_in_middle() {
        let solver = BisectionSolver::new();
        let (a, b) = make_split(16, 7);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        assert_eq!(result.disagreeing_op_index, Some(7));
    }

    #[test]
    fn binary_search_steps_log_n() {
        let solver = BisectionSolver::new();
        let n = 1024;
        let (a, b) = make_split(n, 500);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        // log2(1024) = 10, should take at most 10 steps.
        assert!(result.steps_taken <= 10);
    }

    #[test]
    fn single_op_disagree() {
        let solver = BisectionSolver::new();
        let a = vec!["hash_a".to_string()];
        let b = vec!["hash_b".to_string()];
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        assert_eq!(result.disagreeing_op_index, Some(0));
        assert_eq!(result.steps_taken, 0);
    }

    #[test]
    fn empty_hashes() {
        let solver = BisectionSolver::new();
        let empty: Vec<String> = vec![];
        let result = solver.solve(&empty, &empty, |_| true);
        assert!(!result.found);
        assert_eq!(result.total_ops, 0);
    }

    #[test]
    fn different_lengths() {
        let solver = BisectionSolver::new();
        let a = vec!["h1".to_string()];
        let b = vec!["h1".to_string(), "h2".to_string()];
        let result = solver.solve(&a, &b, |_| true);
        assert!(!result.found);
    }

    #[test]
    fn verify_range_finds_first() {
        let solver = BisectionSolver::new();
        let a = vec!["a".into(), "b".into(), "c".into()];
        let b = vec!["a".into(), "X".into(), "c".into()];
        assert_eq!(solver.verify_range(&a, &b), Some(1));
    }

    #[test]
    fn verify_range_all_match() {
        let solver = BisectionSolver::new();
        let h = vec!["a".into(), "b".into()];
        assert_eq!(solver.verify_range(&h, &h), None);
    }

    #[test]
    fn verify_range_different_lengths() {
        let solver = BisectionSolver::new();
        let a = vec!["a".into()];
        let b = vec!["a".into(), "b".into()];
        assert_eq!(solver.verify_range(&a, &b), None);
    }

    #[test]
    fn result_serde_roundtrip() {
        let result = BisectionResult {
            found: true,
            disagreeing_op_index: Some(5),
            steps_taken: 3,
            total_ops: 100,
            claimant_hash: "abc".into(),
            challenger_hash: "def".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BisectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.found, back.found);
        assert_eq!(result.disagreeing_op_index, back.disagreeing_op_index);
        assert_eq!(result.steps_taken, back.steps_taken);
    }

    #[test]
    fn found_true_has_index() {
        let solver = BisectionSolver::new();
        let (a, b) = make_split(8, 3);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        if result.found {
            assert!(result.disagreeing_op_index.is_some());
        }
    }

    #[test]
    fn large_hash_list() {
        let solver = BisectionSolver::new();
        let n = 10000;
        let (a, b) = make_split(n, 7777);
        let result = solver.solve(&a, &b, |idx| a[idx as usize] == b[idx as usize]);
        assert!(result.found);
        assert_eq!(result.disagreeing_op_index, Some(7777));
        // log2(10000) ~ 14, should take at most 14 steps.
        assert!(result.steps_taken <= 14);
    }
}
