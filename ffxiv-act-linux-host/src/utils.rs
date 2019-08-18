use std::ops::Range;

pub fn find_subsequence<T>(haystack: &[T], needle: &[T], wild_ranges: Option<&Vec<Range<usize>>>) -> Option<usize>
    where T: Eq + Copy
{
    let empty_vec = Vec::new();
    let wild_ranges = &wild_ranges.unwrap_or(&empty_vec);
    haystack.windows(needle.len()).position(|window| {
        matches_with_wildcard(window, needle, wild_ranges)
    })
}

fn matches_with_wildcard<T>(window: &[T], needle: &[T], wild_ranges: &Vec<Range<usize>>) -> bool
    where T: Eq + Copy
{

    if wild_ranges.len() > 0 {
        needle
            .iter()
            .enumerate()
            .filter(|needle_byte|  {
                wild_ranges
                    .iter()
                    .all(|wild_range| !wild_range.contains(&needle_byte.0))
            })
            .all(|needle_byte| window[needle_byte.0] == *needle_byte.1)
    } else {
        window == needle
    }

}