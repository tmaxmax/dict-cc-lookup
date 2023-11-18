use std::cmp::Ordering;

pub fn case_fold_contains(haystack: &str, needle: &str) -> bool {
    let nlen = needle.len();
    let hlen = haystack.len();

    match hlen.cmp(&nlen) {
        Ordering::Less => false,
        Ordering::Equal => case_fold_eq(haystack, needle),
        Ordering::Greater => {
            for i in 0..hlen - nlen {
                if haystack.is_char_boundary(i)
                    && haystack.is_char_boundary(i + nlen)
                    && case_fold_eq(&haystack[i..i + nlen], needle)
                {
                    return true;
                }
            }
            false
        }
    }
}

pub fn case_fold_eq(a: &str, b: &str) -> bool {
    a.chars()
        .zip(b.chars())
        .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
}

pub fn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<U>());
    assert_eq!(std::mem::align_of::<T>(), std::mem::align_of::<U>());
    v.clear();
    v.into_iter().map(|_| unreachable!()).collect()
}
