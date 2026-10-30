//! Deterministic tie order of the frozen tool's libstdc++ introsort. See NOTICE.
// Equal-key order is observable in O0 register plans. Keep median selection,
// partition swaps, the sixteen-item insertion tail and heap fallback together.
pub fn sort_by<T: Copy>(values: &mut [T], less: impl Fn(&T, &T) -> bool) {
    if values.is_empty() {
        return;
    }
    let depth = values.len().ilog2() * 2;
    partition(values, 0, values.len(), depth, &less);
    for index in 1..values.len() {
        let value = values[index];
        let mut hole = index;
        while hole > 0 && less(&value, &values[hole - 1]) {
            values[hole] = values[hole - 1];
            hole -= 1;
        }
        values[hole] = value;
    }
}
fn partition<T: Copy>(
    values: &mut [T],
    first: usize,
    mut past: usize,
    mut depth: u32,
    less: &impl Fn(&T, &T) -> bool,
) {
    while past - first > 16 {
        if depth == 0 {
            heap_sort(&mut values[first..past], less);
            return;
        }
        depth -= 1;
        let a = first + 1;
        let b = first + (past - first) / 2;
        let c = past - 1;
        let median = if less(&values[a], &values[b]) {
            if less(&values[b], &values[c]) {
                b
            } else if less(&values[a], &values[c]) {
                c
            } else {
                a
            }
        } else if less(&values[a], &values[c]) {
            a
        } else if less(&values[b], &values[c]) {
            c
        } else {
            b
        };
        values.swap(first, median);
        let mut left = first + 1;
        let mut right = past;
        loop {
            while less(&values[left], &values[first]) {
                left += 1;
            }
            right -= 1;
            while less(&values[first], &values[right]) {
                right -= 1;
            }
            if left >= right {
                break;
            }
            values.swap(left, right);
            left += 1;
        }
        partition(values, left, past, depth, less);
        past = left;
    }
}
fn heap_sort<T: Copy>(values: &mut [T], less: &impl Fn(&T, &T) -> bool) {
    if values.len() < 2 {
        return;
    }
    for parent in (0..=(values.len() - 2) / 2).rev() {
        adjust(values, parent, values.len(), values[parent], less);
    }
    for past in (2..=values.len()).rev() {
        let value = values[past - 1];
        values[past - 1] = values[0];
        adjust(values, 0, past - 1, value, less);
    }
}
fn adjust<T: Copy>(
    values: &mut [T],
    mut hole: usize,
    len: usize,
    value: T,
    less: &impl Fn(&T, &T) -> bool,
) {
    let top = hole;
    let mut child = hole;
    while child < (len - 1) / 2 {
        child = 2 * (child + 1);
        if less(&values[child], &values[child - 1]) {
            child -= 1;
        }
        values[hole] = values[child];
        hole = child;
    }
    if len.is_multiple_of(2) && child == (len - 2) / 2 {
        child = 2 * (child + 1);
        values[hole] = values[child - 1];
        hole = child - 1;
    }
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(&values[parent], &value) {
            break;
        }
        values[hole] = values[parent];
        hole = parent;
    }
    values[hole] = value;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partition_and_heap_paths_preserve_all_equal_key_identities() {
        for count in 0..128 {
            let original: Vec<_> = (0..count).map(|id| ((id * 37) % 13, id)).collect();
            for heap in [false, true] {
                let mut values = original.clone();
                if heap {
                    heap_sort(&mut values, &|a, b| a.0 < b.0);
                } else {
                    sort_by(&mut values, |a, b| a.0 < b.0);
                }
                assert!(values.windows(2).all(|pair| pair[0].0 <= pair[1].0));
                values.sort_unstable_by_key(|value| value.1);
                assert_eq!(values, original);
            }
        }
    }
}
