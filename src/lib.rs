trait Map<K, V> {
    fn get(key: &K) -> Option<V>;
    fn contains(key: &K) -> bool;
    fn put(key: K, value: V);
    fn remove(key: &K) -> Option<V>;
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
