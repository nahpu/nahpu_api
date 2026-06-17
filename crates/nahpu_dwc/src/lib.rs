pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn add_zeros() {
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    fn add_large_numbers() {
        assert_eq!(add(1000000, 2000000), 3000000);
    }
}
