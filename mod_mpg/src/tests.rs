use super::{calculate_stats, MpgEntry, MpgStats};

#[test]
fn test_stats() {
    assert_eq!(
        calculate_stats(
            200,
            10.0,
            1.50,
            &[MpgEntry {
                mileage: 100,
                fill_litres: 0.0,
                fill_price: 0.0,
                result_price: Some(1.50),
            }],
        ),
        MpgStats {
            total_mileage: 100,
            used_litres: 10.0,
            used_cost: 15.0,

            mpg: 45.4609,
            mpg_us: 37.854118,
            lp100km: 6.2137119,

            perlitre: 1.50,
            permile: 0.15,
            perkm: 0.09320567849999999,

            result_price: 1.50,
        }
    );

    assert_eq!(
        calculate_stats(
            200,
            10.0,
            1.50,
            &[MpgEntry {
                mileage: 100,
                fill_litres: 0.0,
                fill_price: 0.0,
                result_price: Some(1.60),
            }],
        ),
        MpgStats {
            total_mileage: 100,
            used_litres: 10.0,
            used_cost: 16.0,
            mpg: 45.4609,
            mpg_us: 37.854118,
            lp100km: 6.2137119,
            perlitre: 1.6,
            permile: 0.16,
            perkm: 0.0994193904,
            result_price: 1.5846153846153845
        }
    );

    assert_eq!(
        calculate_stats(
            300,
            60.0,
            1.60,
            &[
                MpgEntry {
                    mileage: 299,
                    fill_litres: 1.0,
                    fill_price: 1.50,
                    result_price: None,
                },
                MpgEntry {
                    mileage: 100,
                    fill_litres: 0.0,
                    fill_price: 0.0,
                    result_price: Some(1.60),
                },
            ],
        ),
        MpgStats {
            total_mileage: 200,
            used_litres: 61.0,
            used_cost: 97.59425070688032,
            mpg: 14.9052131147541,
            mpg_us: 12.411186229508196,
            lp100km: 18.951821295,
            perlitre: 1.59990574929312,
            permile: 0.4879712535344016,
            perkm: 0.3032112784944628,
            result_price: 1.5985499891249184
        }
    );

    assert_eq!(
        calculate_stats(
            300,
            45.75,
            1.60,
            &[
                MpgEntry {
                    mileage: 300,
                    fill_litres: 15.25,
                    fill_price: 1.50,
                    result_price: None,
                },
                MpgEntry {
                    mileage: 100,
                    fill_litres: 0.0,
                    fill_price: 0.0,
                    result_price: Some(1.60),
                },
            ],
        ),
        MpgStats {
            total_mileage: 200,
            used_litres: 61.0,
            used_cost: 97.60000000000001,
            mpg: 14.9052131147541,
            mpg_us: 12.411186229508196,
            lp100km: 18.951821295,
            perlitre: 1.6,
            permile: 0.48800000000000004,
            perkm: 0.30322914072,
            result_price: 1.5765384615384614
        }
    );
}
