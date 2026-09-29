use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::{dbpf::ResourceKey, fingerprint::ExactDuplicateGroup};

use super::ResourceOverlapFinding;

const MAX_GROUP_RESOURCE_KEY_SAMPLES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceOverlapGroup {
    pub(crate) classification: String,
    pub(crate) confidence: String,
    pub(crate) counts_toward_attention: bool,
    pub(crate) file_ids: Vec<i64>,
    pub(crate) relative_paths: Vec<String>,
    pub(crate) overlap_pair_count: u64,
    pub(crate) shared_resource_count: u64,
    pub(crate) sample_resource_keys: Vec<ResourceKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConflictAggregation {
    pub(crate) exact_duplicate_group_count: u64,
    pub(crate) potential_conflict_group_count: u64,
    pub(crate) attention_group_count: u64,
    pub(crate) raw_overlap_pair_count: u64,
    pub(crate) suppressed_duplicate_overlap_pair_count: u64,
    pub(crate) potential_conflict_groups: Vec<ResourceOverlapGroup>,
}

pub(crate) fn aggregate_findings(
    exact_duplicates: &[ExactDuplicateGroup],
    resource_overlaps: &[ResourceOverlapFinding],
) -> ConflictAggregation {
    let duplicate_membership = duplicate_membership(exact_duplicates);
    let mut adjacency: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
    let mut paths: BTreeMap<i64, String> = BTreeMap::new();
    let mut kept_edges = Vec::new();
    let mut suppressed_duplicate_overlap_pair_count = 0_u64;

    for overlap in resource_overlaps {
        let same_duplicate_group = duplicate_membership
            .get(&overlap.left_file_id)
            .zip(duplicate_membership.get(&overlap.right_file_id))
            .is_some_and(|(left, right)| left == right);

        if same_duplicate_group {
            suppressed_duplicate_overlap_pair_count += 1;
            continue;
        }

        adjacency
            .entry(overlap.left_file_id)
            .or_default()
            .insert(overlap.right_file_id);
        adjacency
            .entry(overlap.right_file_id)
            .or_default()
            .insert(overlap.left_file_id);

        paths
            .entry(overlap.left_file_id)
            .or_insert_with(|| overlap.left_relative_path.clone());
        paths
            .entry(overlap.right_file_id)
            .or_insert_with(|| overlap.right_relative_path.clone());
        kept_edges.push(overlap);
    }

    let mut groups = Vec::new();
    let mut visited = BTreeSet::new();

    for start in adjacency.keys().copied() {
        if !visited.insert(start) {
            continue;
        }

        let mut queue = VecDeque::from([start]);
        let mut component = BTreeSet::from([start]);

        while let Some(file_id) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&file_id) {
                for neighbor in neighbors {
                    if visited.insert(*neighbor) {
                        component.insert(*neighbor);
                        queue.push_back(*neighbor);
                    }
                }
            }
        }

        let mut pair_count = 0_u64;
        let mut shared_resource_count = 0_u64;
        let mut sample_keys = BTreeSet::new();

        for overlap in &kept_edges {
            if component.contains(&overlap.left_file_id)
                && component.contains(&overlap.right_file_id)
            {
                pair_count += 1;
                shared_resource_count =
                    shared_resource_count.saturating_add(overlap.shared_resource_count);
                for key in &overlap.sample_resource_keys {
                    if sample_keys.len() >= MAX_GROUP_RESOURCE_KEY_SAMPLES {
                        break;
                    }
                    sample_keys.insert(*key);
                }
            }
        }

        let file_ids = component.into_iter().collect::<Vec<_>>();
        let relative_paths = file_ids
            .iter()
            .filter_map(|file_id| paths.get(file_id).cloned())
            .collect::<Vec<_>>();

        groups.push(ResourceOverlapGroup {
            classification: "potentialConflictGroup".to_string(),
            confidence: "low".to_string(),
            counts_toward_attention: false,
            file_ids,
            relative_paths,
            overlap_pair_count: pair_count,
            shared_resource_count,
            sample_resource_keys: sample_keys.into_iter().collect(),
        });
    }

    groups.sort_by(|left, right| {
        right
            .shared_resource_count
            .cmp(&left.shared_resource_count)
            .then_with(|| right.overlap_pair_count.cmp(&left.overlap_pair_count))
            .then_with(|| left.file_ids.cmp(&right.file_ids))
    });

    ConflictAggregation {
        exact_duplicate_group_count: exact_duplicates.len() as u64,
        potential_conflict_group_count: groups.len() as u64,
        attention_group_count: exact_duplicates.len() as u64,
        raw_overlap_pair_count: resource_overlaps.len() as u64,
        suppressed_duplicate_overlap_pair_count,
        potential_conflict_groups: groups,
    }
}

fn duplicate_membership(exact_duplicates: &[ExactDuplicateGroup]) -> BTreeMap<i64, usize> {
    let mut membership = BTreeMap::new();

    for (group_index, group) in exact_duplicates.iter().enumerate() {
        for file in &group.files {
            membership.insert(file.local_file_id, group_index);
        }
    }

    membership
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: u64) -> ResourceKey {
        ResourceKey {
            resource_type: 1,
            group: 1,
            instance: value,
        }
    }

    fn overlap(left: i64, right: i64, shared_resource_count: u64) -> ResourceOverlapFinding {
        ResourceOverlapFinding {
            classification: "potentialConflict".to_string(),
            left_file_id: left,
            left_relative_path: format!("Creator/item-{left:05}.package"),
            right_file_id: right,
            right_relative_path: format!("Creator/item-{right:05}.package"),
            shared_resource_count,
            sample_resource_keys: vec![key((left as u64) << 32 | right as u64)],
        }
    }

    #[test]
    fn pairwise_overlap_edges_collapse_into_connected_action_groups() {
        let raw = vec![
            overlap(1, 2, 4),
            overlap(2, 3, 3),
            overlap(10, 11, 2),
            overlap(11, 12, 2),
            overlap(10, 12, 1),
        ];

        let aggregation = aggregate_findings(&[], &raw);

        assert_eq!(aggregation.raw_overlap_pair_count, 5);
        assert_eq!(aggregation.potential_conflict_group_count, 2);
        assert_eq!(aggregation.attention_group_count, 0);
        assert!(aggregation
            .potential_conflict_groups
            .iter()
            .all(|group| group.confidence == "low"));
        assert!(aggregation
            .potential_conflict_groups
            .iter()
            .all(|group| !group.counts_toward_attention));

        let first = &aggregation.potential_conflict_groups[0];
        assert_eq!(first.file_ids, vec![1, 2, 3]);
        assert_eq!(first.overlap_pair_count, 2);
        assert_eq!(first.shared_resource_count, 7);
    }

    #[test]
    fn dogfood_shape_reduces_ten_thousand_raw_pairs_to_one_hundred_groups() {
        let mut raw = Vec::with_capacity(10_000);

        for cluster in 0..100_i64 {
            let base = cluster * 50;
            for edge in 0..100_i64 {
                let left = base + (edge % 50) + 1;
                let right = if edge % 50 == 49 {
                    base + 1
                } else {
                    left + 1
                };
                raw.push(overlap(left, right, 1));
            }
        }

        assert_eq!(raw.len(), 10_000);

        let aggregation = aggregate_findings(&[], &raw);

        assert_eq!(aggregation.raw_overlap_pair_count, 10_000);
        assert_eq!(aggregation.potential_conflict_group_count, 100);
        assert_eq!(aggregation.attention_group_count, 0);
        assert!(
            aggregation.potential_conflict_group_count
                <= aggregation.raw_overlap_pair_count / 100,
            "top-level potential-conflict groups should be at least 100x quieter than raw pair evidence"
        );
    }

    #[test]
    #[ignore = "5k-library conflict aggregation benchmark; run in scanner benchmark workflow"]
    fn benchmark_5000_file_conflict_aggregation() {
        let mut raw = Vec::with_capacity(10_000);

        for cluster in 0..100_i64 {
            let base = cluster * 50;
            for edge in 0..100_i64 {
                let left = base + (edge % 50) + 1;
                let right = if edge % 50 == 49 {
                    base + 1
                } else {
                    left + 1
                };
                raw.push(overlap(left, right, 1));
            }
        }

        let started = std::time::Instant::now();
        let aggregation = aggregate_findings(&[], &raw);
        let elapsed = started.elapsed();

        eprintln!(
            "CONFLICT_AGGREGATION_BENCHMARK files=5000 raw_pairs={} potential_groups={} attention_groups={} grouping_ms={}",
            aggregation.raw_overlap_pair_count,
            aggregation.potential_conflict_group_count,
            aggregation.attention_group_count,
            elapsed.as_millis()
        );

        assert_eq!(aggregation.potential_conflict_group_count, 100);
        assert_eq!(aggregation.attention_group_count, 0);
    }
}
