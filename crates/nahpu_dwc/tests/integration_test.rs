use nahpu_dwc;

#[test]
fn test_add_integration() {
    // A simple test to verify the crate can be used as a dependency
    let sum = nahpu_dwc::add(10, 20);
    assert_eq!(sum, 30);
}
