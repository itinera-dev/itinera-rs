//! Grouping and counting what admission finds.

use std::collections::BTreeMap;

/// Groups values by key, keeping the values of each key in order.
pub(super) fn grouped<K: Ord, V>(pairs: impl Iterator<Item = (K, V)>) -> BTreeMap<K, Vec<V>> {
    pairs.fold(BTreeMap::new(), add_to_group)
}

fn add_to_group<K: Ord, V>(
    mut groups: BTreeMap<K, Vec<V>>,
    (key, value): (K, V),
) -> BTreeMap<K, Vec<V>> {
    groups.entry(key).or_default().push(value);
    groups
}

pub(super) fn more_than_once<K, V>((_, values): &(K, Vec<V>)) -> bool {
    values.len() > 1
}

/// Counts how many times each key occurs.
pub(super) fn counted<K: Ord>(keys: impl Iterator<Item = K>) -> BTreeMap<K, usize> {
    keys.fold(BTreeMap::new(), add_to_count)
}

fn add_to_count<K: Ord>(mut counts: BTreeMap<K, usize>, key: K) -> BTreeMap<K, usize> {
    *counts.entry(key).or_default() += 1;
    counts
}

pub(super) fn repeated<K>((_, count): &(K, usize)) -> bool {
    *count > 1
}
