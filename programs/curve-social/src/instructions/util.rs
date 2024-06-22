
pub fn calculate_fee(
    amount: u64,
    fee_basis_points: u64,
) -> u64 {
    amount * fee_basis_points / 10000
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fee() {
        assert_eq!(calculate_fee(100, 100), 1); //1% fee
        assert_eq!(calculate_fee(100, 1000), 10); //10% fee
        assert_eq!(calculate_fee(100, 5000), 50); //50% fee
        assert_eq!(calculate_fee(100, 50000), 500); //500% fee
        assert_eq!(calculate_fee(100, 50), 0); //0.5% fee 
        assert_eq!(calculate_fee(1000, 50), 5); //0.5% fee
        assert_eq!(calculate_fee(100, 0), 0); //0% fee
    }
}