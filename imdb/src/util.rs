use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct NonNan(f64);

impl NonNan {
    pub fn new(val: f64) -> NonNan {
        if val.is_nan() {
            panic!("NonNan created with NaN value");
        }
        NonNan(val)
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    #[inline]
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Deref for NonNan {
    type Target = f64;

    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

#[derive(Debug)]
pub struct Counter<K: Hash + Eq> {
    inner: HashMap<K, u32>,
}

impl<K> Counter<K>
where
    K: Hash + Eq,
{
    pub fn new() -> Counter<K> {
        Counter {
            inner: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: K) {
        *self.inner.entry(key).or_insert(0) += 1;
    }

    pub fn most_common(&self) -> Vec<&K> {
        let mut most_common = Vec::new();
        let mut most_count = 0;
        for (key, &count) in self.inner.iter() {
            if count == most_count {
                most_common.push(key);
            } else if count >= most_count {
                most_common.clear();
                most_common.push(key);
                most_count = count;
            }
        }
        most_common
    }
}

#[test]
fn test_most_common() {
    let mut c = Counter::new();
    c.add("hello");
    assert_eq!(c.most_common(), vec![&"hello"]);
    c.add("hey");
    assert_eq!(c.most_common().len(), 2);
    c.add("hello");
    assert_eq!(c.most_common(), vec![&"hello"]);
}
