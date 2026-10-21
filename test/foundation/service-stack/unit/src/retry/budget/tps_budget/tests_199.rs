#[cfg(test)]
mod tests {
    use crate::retry::budget::Budget;

    use super::*;
    use tokio::time;

    #[test]
    fn tps_empty() {
        let bgt = TpsBudget::new(Duration::from_secs(1), 0, 1.0);
        assert!(!bgt.withdraw());
    }

    #[tokio::test]
    async fn tps_leaky() {
        time::pause();

        let bgt = TpsBudget::new(Duration::from_secs(1), 0, 1.0);
        bgt.deposit();

        time::advance(Duration::from_secs(3)).await;

        assert!(!bgt.withdraw());
    }

    #[tokio::test]
    async fn tps_slots() {
        time::pause();

        let bgt = TpsBudget::new(Duration::from_secs(1), 0, 0.5);
        bgt.deposit();
        bgt.deposit();
        time::advance(Duration::from_millis(901)).await;
        // 900ms later, the deposit should still be valid
        assert!(bgt.withdraw());

        // blank slate
        time::advance(Duration::from_millis(2001)).await;

        bgt.deposit();
        time::advance(Duration::from_millis(301)).await;
        bgt.deposit();
        time::advance(Duration::from_millis(801)).await;
        bgt.deposit();

        // the first deposit is expired, but the 2nd should still be valid,
        // combining with the 3rd
        assert!(bgt.withdraw());
    }

    #[tokio::test]
    async fn tps_reserve() {
        let bgt = TpsBudget::new(Duration::from_secs(1), 5, 1.0);
        assert!(bgt.withdraw());
        assert!(bgt.withdraw());
        assert!(bgt.withdraw());
        assert!(bgt.withdraw());
        assert!(bgt.withdraw());

        assert!(!bgt.withdraw());
    }
}
