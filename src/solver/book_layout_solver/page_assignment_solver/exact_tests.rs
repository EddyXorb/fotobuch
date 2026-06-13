//! Exactness, weight-isolation and performance tests for the page-assignment DP.

use super::super::model::GroupInfo;
use super::{PageAssignmentError, solve_exact};
use crate::dto_models::BookLayoutSolverConfig as Params;
use std::time::{Duration, Instant};

fn default_params() -> Params {
    Params {
        page_target: 2,
        page_min: 1,
        page_max: 5,
        photos_per_page_min: 3,
        photos_per_page_max: 10,
        group_max_per_page: 3,
        group_min_photos: 3,
        weight_even: 1.0,
        weight_split: 1.0,
        weight_pages: 1.0,
        search_timeout: Duration::from_secs(10),
        max_coverage_cost: 0.1,
        enable_local_search: true,
        mip_rel_gap: None,
        max_photos_for_split: None,
        split_group_boundary_slack: None,
    }
}

/// Independent reference implementation of the objective Z from dp.typ.
/// Returns `None` if the cut vector is infeasible.
fn reference_cost(cuts: &[usize], groups: &GroupInfo, p: &Params) -> Option<f64> {
    let n = groups.total_photos();
    let m = cuts.len() - 1;
    if m < p.page_min || m > p.page_max {
        return None;
    }
    // Even target is relative to the actual page count m, not the target s.
    let n_bar = n as f64 / m as f64;
    let pg = |i: usize| groups.group_of_photo(i);

    let mut total = 0.0;
    for j in 0..m {
        let (a, b) = (cuts[j], cuts[j + 1]);
        let size = b - a;
        if size < p.photos_per_page_min || size > p.photos_per_page_max {
            return None;
        }
        if pg(b - 1) - pg(a) + 1 > p.group_max_per_page {
            return None;
        }
        for l in [pg(a), pg(b - 1)] {
            let range = groups.group_range(l);
            let fragment = b.min(range.end) - a.max(range.start);
            let gs = groups.group_size(l);
            if fragment < gs && !(gs >= p.group_min_photos && fragment >= p.group_min_photos) {
                return None;
            }
        }
        total += p.weight_even * (size as f64 - n_bar).abs();
    }
    for &c in &cuts[1..m] {
        if pg(c - 1) == pg(c) {
            total += p.weight_split;
        }
    }
    total += p.weight_pages * (m as f64 - p.page_target as f64).abs();
    Some(total)
}

/// Brute-force optimum over all cut vectors (only for small n).
fn brute_force_min(groups: &GroupInfo, p: &Params) -> Option<f64> {
    let n = groups.total_photos();
    assert!(n <= 16, "brute force only for small instances");
    let mut best: Option<f64> = None;
    for mask in 0u32..(1u32 << (n - 1)) {
        let mut cuts = vec![0];
        for pos in 1..n {
            if mask & (1 << (pos - 1)) != 0 {
                cuts.push(pos);
            }
        }
        cuts.push(n);
        if let Some(cost) = reference_cost(&cuts, groups, p) {
            best = Some(best.map_or(cost, |b| b.min(cost)));
        }
    }
    best
}

// --- Ported simple instances (from the former MIP tests) ---

#[test]
fn test_solve_simple_two_groups() {
    let groups = GroupInfo::new(&[5, 5]);
    let assignment = solve_exact(&groups, &default_params()).unwrap();
    assert_eq!(assignment.num_pages(), 2);
    assert_eq!(assignment.total_photos(), 10);
}

#[test]
fn test_solve_three_groups() {
    let groups = GroupInfo::new(&[4, 5, 6]);
    let params = Params {
        page_target: 3,
        page_min: 2,
        page_max: 5,
        photos_per_page_min: 4,
        photos_per_page_max: 6,
        group_max_per_page: 2,
        group_min_photos: 3,
        weight_split: 10.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    assert_eq!(assignment.total_photos(), 15);
    assert!(assignment.num_pages() >= params.page_min);
    assert!(assignment.num_pages() <= params.page_max);
}

#[test]
fn test_solve_respects_page_sizes() {
    let groups = GroupInfo::new(&[8, 2]);
    let params = Params {
        page_target: 2,
        page_min: 2,
        page_max: 3,
        photos_per_page_min: 3,
        photos_per_page_max: 6,
        group_max_per_page: 2,
        group_min_photos: 3,
        weight_split: 0.1,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    for page in 0..assignment.num_pages() {
        let size = assignment.page_size(page);
        assert!(size >= params.photos_per_page_min);
        assert!(size <= params.photos_per_page_max);
    }
}

// --- Weight isolation tests ---

/// w1 dominant, three equal groups: unique optimum is 3 pages of 3 (D_even=0).
#[test]
fn test_weight_even_only_produces_equal_pages() {
    let groups = GroupInfo::new(&[3, 3, 3]);
    let params = Params {
        page_target: 3,
        page_min: 2,
        page_max: 5,
        photos_per_page_min: 1,
        photos_per_page_max: 9,
        group_max_per_page: 3,
        group_min_photos: 1,
        weight_even: 1000.0,
        weight_split: 0.0,
        weight_pages: 0.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    assert_eq!(assignment.num_pages(), 3);
    for i in 0..3 {
        assert_eq!(assignment.page_size(i), 3);
    }
}

/// w1 dominant, single group of 9: optimum splits evenly into 3×3.
#[test]
fn test_weight_even_only_splits_single_group_evenly() {
    let groups = GroupInfo::new(&[9]);
    let params = Params {
        page_target: 3,
        page_min: 2,
        page_max: 5,
        photos_per_page_min: 2,
        photos_per_page_max: 5,
        group_max_per_page: 1,
        group_min_photos: 1,
        weight_even: 1000.0,
        weight_split: 0.0,
        weight_pages: 0.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    assert_eq!(assignment.total_photos(), 9);
    for i in 0..assignment.num_pages() {
        assert_eq!(assignment.page_size(i), 3);
    }
}

/// w2 dominant: no group is ever split (every internal cut is a group boundary).
#[test]
fn test_weight_split_only_keeps_groups_together() {
    let groups = GroupInfo::new(&[5, 4]);
    let params = Params {
        page_target: 2,
        page_min: 1,
        page_max: 4,
        photos_per_page_min: 1,
        photos_per_page_max: 9,
        group_max_per_page: 2,
        group_min_photos: 2,
        weight_even: 0.0,
        weight_split: 1000.0,
        weight_pages: 0.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    let cuts = assignment.cuts();
    for &c in &cuts[1..cuts.len() - 1] {
        assert_ne!(
            groups.group_of_photo(c - 1),
            groups.group_of_photo(c),
            "internal cut at {c} splits a group despite huge weight_split"
        );
    }
}

/// w3 dominant: page count lands exactly on the target.
#[test]
fn test_weight_pages_only_hits_target_page_count() {
    let groups = GroupInfo::new(&[9]);
    let params = Params {
        page_target: 2,
        page_min: 1,
        page_max: 5,
        photos_per_page_min: 1,
        photos_per_page_max: 9,
        group_max_per_page: 1,
        group_min_photos: 1,
        weight_even: 0.0,
        weight_split: 0.0,
        weight_pages: 1000.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    assert_eq!(assignment.num_pages(), 2);
}

/// Even-vs-split tradeoff on groups [6, 2], n̄=4, with a small page nudge.
#[test]
fn test_weight_even_vs_split_tradeoff() {
    let base = Params {
        page_target: 2,
        page_min: 2,
        page_max: 3,
        photos_per_page_min: 1,
        photos_per_page_max: 8,
        group_max_per_page: 2,
        group_min_photos: 1,
        weight_even: 0.0,
        weight_split: 0.0,
        weight_pages: 0.0,
        ..default_params()
    };

    // High w1: split group 1 into [4 | 2+2] → pages [4, 4].
    let even = solve_exact(
        &GroupInfo::new(&[6, 2]),
        &Params {
            weight_even: 1000.0,
            weight_pages: 1.0,
            ..base.clone()
        },
    )
    .unwrap();
    for i in 0..even.num_pages() {
        assert_eq!(even.page_size(i), 4);
    }

    // High w2: keep groups intact → pages [6, 2].
    let split = solve_exact(
        &GroupInfo::new(&[6, 2]),
        &Params {
            weight_split: 1000.0,
            weight_pages: 1.0,
            ..base
        },
    )
    .unwrap();
    assert_eq!(split.num_pages(), 2);
    let sizes: Vec<usize> = (0..split.num_pages()).map(|i| split.page_size(i)).collect();
    assert!(sizes.contains(&6) && sizes.contains(&2), "got {sizes:?}");
}

/// The even term targets the *actual* page count (n̄ = n / m), not the target s.
/// So even (w1) and page-count (w3) weights pull independently: with the same
/// instance and target, the even-driven optimum uses a different page count than
/// the page-count-driven optimum.
#[test]
fn test_even_targets_actual_page_count_not_target() {
    // Single group of 15, pages of size 1..=6. Equal-size splits exist only for
    // m ∈ {3, 5} (sizes 5 or 3); m = 6 cannot be made even.
    let groups = GroupInfo::new(&[15]);
    let base = Params {
        page_target: 6,
        page_min: 1,
        page_max: 15,
        photos_per_page_min: 1,
        photos_per_page_max: 6,
        group_max_per_page: 1,
        group_min_photos: 1,
        ..default_params()
    };

    // w1 = 0 → free page count, page term hits the target exactly: 6 pages.
    let by_pages = solve_exact(
        &groups,
        &Params {
            weight_even: 0.0,
            weight_split: 0.0,
            weight_pages: 1000.0,
            ..base.clone()
        },
    )
    .unwrap();
    assert_eq!(by_pages.num_pages(), 6);

    // w1 dominant, weak page nudge → even forces an equal split (m ∈ {3, 5}); the
    // page term breaks the tie towards the target 6, selecting m = 5 (not 6).
    let by_even = solve_exact(
        &groups,
        &Params {
            weight_even: 1000.0,
            weight_split: 0.0,
            weight_pages: 1.0,
            ..base
        },
    )
    .unwrap();
    assert_eq!(by_even.num_pages(), 5);
    for i in 0..by_even.num_pages() {
        assert_eq!(by_even.page_size(i), 3);
    }
}

// --- Exactness, infeasibility, determinism, performance ---

#[test]
fn test_exactness_against_brute_force() {
    let cases: Vec<(GroupInfo, Params)> = vec![
        (GroupInfo::new(&[9]), default_params()),
        (GroupInfo::new(&[4, 5]), default_params()),
        (
            GroupInfo::new(&[3, 3, 3]),
            Params {
                page_target: 3,
                page_min: 1,
                page_max: 5,
                photos_per_page_min: 1,
                photos_per_page_max: 6,
                group_max_per_page: 2,
                group_min_photos: 1,
                weight_even: 1.0,
                weight_split: 7.0,
                weight_pages: 2.0,
                ..default_params()
            },
        ),
        (
            GroupInfo::new(&[6, 2, 4]),
            Params {
                page_target: 3,
                page_min: 2,
                page_max: 6,
                photos_per_page_min: 1,
                photos_per_page_max: 5,
                group_max_per_page: 2,
                group_min_photos: 2,
                weight_even: 3.0,
                weight_split: 1.0,
                weight_pages: 4.0,
                ..default_params()
            },
        ),
    ];

    for (groups, params) in &cases {
        let assignment = solve_exact(groups, params).expect("instance should be feasible");
        let dp_cost =
            reference_cost(assignment.cuts(), groups, params).expect("dp solution feasible");
        let bf = brute_force_min(groups, params).expect("brute force found an optimum");
        assert!(
            (dp_cost - bf).abs() < 1e-9,
            "dp_cost={dp_cost}, brute_force={bf}"
        );
    }
}

#[test]
fn test_infeasible_unsplittable_oversized_group() {
    // Group of 6 cannot fit (p_max=4) nor split (fragment must be >= g_min=6).
    let groups = GroupInfo::new(&[6]);
    let params = Params {
        page_target: 2,
        page_min: 1,
        page_max: 3,
        photos_per_page_min: 1,
        photos_per_page_max: 4,
        group_max_per_page: 1,
        group_min_photos: 6,
        ..default_params()
    };

    assert!(matches!(
        solve_exact(&groups, &params),
        Err(PageAssignmentError::Infeasible)
    ));
}

#[test]
fn test_infeasible_empty_instance() {
    let groups = GroupInfo::new(&[]);
    assert!(matches!(
        solve_exact(&groups, &default_params()),
        Err(PageAssignmentError::Infeasible)
    ));
}

#[test]
fn test_determinism() {
    let groups = GroupInfo::new(&[6, 2, 4]);
    let params = Params {
        page_target: 3,
        page_min: 2,
        page_max: 6,
        photos_per_page_min: 1,
        photos_per_page_max: 5,
        group_max_per_page: 2,
        group_min_photos: 1,
        weight_even: 2.0,
        weight_split: 3.0,
        weight_pages: 1.0,
        ..default_params()
    };

    let a = solve_exact(&groups, &params).unwrap();
    let b = solve_exact(&groups, &params).unwrap();
    assert_eq!(a.cuts(), b.cuts());
}

/// Cut exactly on a group boundary must not incur weight_split.
#[test]
fn test_boundary_cut_is_free() {
    let groups = GroupInfo::new(&[4, 4]);
    let params = Params {
        page_target: 2,
        page_min: 1,
        page_max: 4,
        photos_per_page_min: 1,
        photos_per_page_max: 8,
        group_max_per_page: 2,
        group_min_photos: 1,
        weight_even: 1.0,
        weight_split: 1000.0,
        // Nudge towards the 2-page target so a single page (also perfectly even
        // under n̄ = n / m) does not tie the boundary split.
        weight_pages: 1.0,
        ..default_params()
    };

    let assignment = solve_exact(&groups, &params).unwrap();
    // Optimum splits at the boundary (index 4): even pages, zero split cost.
    assert_eq!(assignment.cuts(), &[0, 4, 8]);
    let cost = reference_cost(assignment.cuts(), &groups, &params).unwrap();
    assert!(cost < params.weight_split, "boundary cut wrongly penalised");
}

#[test]
fn test_performance_large_instance() {
    // 1000 photos in 30 groups: must solve well under the bound. The solver runs
    // one inner DP per feasible page count b ∈ B (here b ∈ [62, 72]), so runtime
    // scales with the page-count window |B|; a realistic book keeps it narrow.
    let group_sizes: Vec<usize> = (0..30).map(|i| if i < 10 { 34 } else { 33 }).collect();
    let groups = GroupInfo::new(&group_sizes);
    assert_eq!(groups.total_photos(), 1000);

    let params = Params {
        page_target: 67,
        page_min: 62,
        page_max: 72,
        photos_per_page_min: 1,
        photos_per_page_max: 20,
        group_max_per_page: 5,
        group_min_photos: 1,
        weight_even: 1.0,
        weight_split: 10.0,
        weight_pages: 5.0,
        ..default_params()
    };

    let start = Instant::now();
    let assignment = solve_exact(&groups, &params).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(assignment.total_photos(), 1000);
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
}
