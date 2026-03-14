use derive_getters::Getters;
use std::ops::Add;
use std::ops::AddAssign;

#[derive(Debug, Eq, Copy, PartialEq, Clone, Getters)]
pub struct Quantity {
    value: u32,
}

impl Quantity {
    // TODO: add English comment
    // TODO: add English comment
    const MAX: u32 = 10_000_000;

    // TODO: add English comment
    pub fn new(value: u32) -> Self {
        if value > Quantity::MAX {
            panic!("Quantityは {}超の値にならない", Quantity::MAX);
        }
        Quantity { value }
    }
}

impl Add for Quantity {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let sum = self
            .value
            .checked_add(other.value)
            .expect("Quantity addition overflowed");
        if sum > Quantity::MAX {
            panic!(
                "Quantity addition resulted in a value exceeding {}",
                Quantity::MAX
            );
        }
        Self::new(sum)
    }
}
impl AddAssign for Quantity {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(Quantity::new(500), Quantity::new(400), Quantity::new(900))]
    #[case(
        Quantity::new(500_000),
        Quantity::new(400_000),
        Quantity::new(900_000)
    )]
    fn test_addition(
        #[case] q1: Quantity,
        #[case] q2: Quantity,
        #[case] expected: Quantity,
    ) {
        let result = q1 + q2;
        assert_eq!(result, expected);
    }
    #[rstest]
    #[case(
        Quantity::new(500_000),
        Quantity::new(400_000),
        Quantity::new(900_000)
    )]
    fn test_add_assign(
        #[case] mut q1: Quantity,
        #[case] q2: Quantity,
        #[case] expected: Quantity,
    ) {
        q1 += q2;
        assert_eq!(q1, expected);
    }

    #[rstest]
    #[should_panic]
    #[case(Quantity::new(5_000_000), Quantity::new(8_000_000))]
    fn test_addition_overflow(#[case] q1: Quantity, #[case] q2: Quantity) {
        let _result = q1 + q2;
    }

    #[rstest]
    #[should_panic]
    #[case(50_000_000)]
    #[case(0)]
    fn test_validate(#[case] q1: u32) {
        let _q = Quantity::new(q1);
    }
}
