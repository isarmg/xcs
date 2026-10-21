#[cfg(test)]
mod tests {
    use super::*;
    use error::DivisionError;
    use ::numeric::collection::Zero;

    // mod divide_to_string_u8;

    #[cfg(feature = "with-bigint")]
    use ::numeric::collection::{bigint::BigUint, Num};

    #[test]
    fn test_division() {
        const PRECISION: usize = 64;
        let data: Vec<(u8, u8, &'static str, bool)> = vec![
            (
                1u8,
                255u8,
                "0.0039215686274509803921568627450980392156862745098039215686274509",
                false,
            ),
            (
                1u8,
                254u8,
                "0.0039370078740157480314960629921259842519685039370078740157480314",
                false,
            ),
            (
                1u8,
                253u8,
                "0.003952569169960474308300395256916996047430830039525691699604743",
                false,
            ),
            (
                1u8,
                252u8,
                "0.0039682539682539682539682539682539682539682539682539682539682539",
                false,
            ),
            (
                1u8,
                251u8,
                "0.0039840637450199203187250996015936254980079681274900398406374501",
                false,
            ),
            (1u8, 250u8, "0.004", true),
            (
                1u8,
                112u8,
                "0.0089285714285714285714285714285714285714285714285714285714285714",
                false,
            ),
            (
                1u8,
                111u8,
                "0.009009009009009009009009009009009009009009009009009009009009009",
                false,
            ),
            (
                1u8,
                96u8,
                "0.0104166666666666666666666666666666666666666666666666666666666666",
                false,
            ),
            (
                1u8,
                92u8,
                "0.0108695652173913043478260869565217391304347826086956521739130434",
                false,
            ),
            (
                1u8,
                91u8,
                "0.0109890109890109890109890109890109890109890109890109890109890109",
                false,
            ),
            (
                1u8,
                90u8,
                "0.0111111111111111111111111111111111111111111111111111111111111111",
                false,
            ),
            (
                1u8,
                9u8,
                "0.1111111111111111111111111111111111111111111111111111111111111111",
                false,
            ),
            (1u8, 8u8, "0.125", true),
            (
                1u8,
                7u8,
                "0.1428571428571428571428571428571428571428571428571428571428571428",
                false,
            ),
            (
                1u8,
                6u8,
                "0.1666666666666666666666666666666666666666666666666666666666666666",
                false,
            ),
            (1u8, 5u8, "0.2", true),
            (1u8, 4u8, "0.25", true),
            (
                1u8,
                3u8,
                "0.3333333333333333333333333333333333333333333333333333333333333333",
                false,
            ),
            (1u8, 2u8, "0.5", true),
            (1u8, 1u8, "1", true),
            (
                49u8,
                255u8,
                "0.192156862745098039215686274509803921568627450980392156862745098",
                false,
            ),
            (
                49u8,
                254u8,
                "0.1929133858267716535433070866141732283464566929133858267716535433",
                false,
            ),
            (
                49u8,
                253u8,
                "0.193675889328063241106719367588932806324110671936758893280632411",
                false,
            ),
            (
                49u8,
                252u8,
                "0.1944444444444444444444444444444444444444444444444444444444444444",
                false,
            ),
            (
                49u8,
                251u8,
                "0.1952191235059760956175298804780876494023904382470119521912350597",
                false,
            ),
            (49u8, 250u8, "0.196", true),
            (
                49u8,
                249u8,
                "0.1967871485943775100401606425702811244979919678714859437751004016",
                false,
            ),
            (
                49u8,
                248u8,
                "0.1975806451612903225806451612903225806451612903225806451612903225",
                false,
            ),
            (
                49u8,
                247u8,
                "0.1983805668016194331983805668016194331983805668016194331983805668",
                false,
            ),
            (
                49u8,
                246u8,
                "0.1991869918699186991869918699186991869918699186991869918699186991",
                false,
            ),
            (49u8, 245u8, "0.2", true),
            (
                49u8,
                69u8,
                "0.7101449275362318840579710144927536231884057971014492753623188405",
                false,
            ),
            (
                49u8,
                68u8,
                "0.7205882352941176470588235294117647058823529411764705882352941176",
                false,
            ),
            (
                49u8,
                67u8,
                "0.7313432835820895522388059701492537313432835820895522388059701492",
                false,
            ),
            (
                49u8,
                66u8,
                "0.7424242424242424242424242424242424242424242424242424242424242424",
                false,
            ),
            (
                49u8,
                65u8,
                "0.7538461538461538461538461538461538461538461538461538461538461538",
                false,
            ),
            (49u8, 64u8, "0.765625", true),
            (
                49u8,
                63u8,
                "0.7777777777777777777777777777777777777777777777777777777777777777",
                false,
            ),
            (
                77u8,
                255u8,
                "0.3019607843137254901960784313725490196078431372549019607843137254",
                false,
            ),
            (
                77u8,
                254u8,
                "0.3031496062992125984251968503937007874015748031496062992125984251",
                false,
            ),
            (
                77u8,
                253u8,
                "0.3043478260869565217391304347826086956521739130434782608695652173",
                false,
            ),
            (
                77u8,
                252u8,
                "0.3055555555555555555555555555555555555555555555555555555555555555",
                false,
            ),
            (
                77u8,
                251u8,
                "0.3067729083665338645418326693227091633466135458167330677290836653",
                false,
            ),
            (77u8, 250u8, "0.308", true),
            (
                77u8,
                249u8,
                "0.3092369477911646586345381526104417670682730923694779116465863453",
                false,
            ),
            (
                77u8,
                248u8,
                "0.3104838709677419354838709677419354838709677419354838709677419354",
                false,
            ),
            (
                77u8,
                231u8,
                "0.3333333333333333333333333333333333333333333333333333333333333333",
                false,
            ),
            (
                77u8,
                230u8,
                "0.3347826086956521739130434782608695652173913043478260869565217391",
                false,
            ),
            (
                77u8,
                229u8,
                "0.3362445414847161572052401746724890829694323144104803493449781659",
                false,
            ),
            (
                77u8,
                228u8,
                "0.3377192982456140350877192982456140350877192982456140350877192982",
                false,
            ),
            (
                77u8,
                227u8,
                "0.3392070484581497797356828193832599118942731277533039647577092511",
                false,
            ),
            (
                77u8,
                226u8,
                "0.3407079646017699115044247787610619469026548672566371681415929203",
                false,
            ),
            (
                77u8,
                225u8,
                "0.3422222222222222222222222222222222222222222222222222222222222222",
                false,
            ),
            (77u8, 224u8, "0.34375", true),
            (
                77u8,
                223u8,
                "0.3452914798206278026905829596412556053811659192825112107623318385",
                false,
            ),
            (
                77u8,
                222u8,
                "0.3468468468468468468468468468468468468468468468468468468468468468",
                false,
            ),
            (
                77u8,
                221u8,
                "0.3484162895927601809954751131221719457013574660633484162895927601",
                false,
            ),
            (77u8, 220u8, "0.35", true),
        ];

        for i in data.iter() {
            assert_eq!(divide_to_string(i.0, i.1, PRECISION, false).unwrap(), i.2);

            {
                let mut string: String = String::new();
                let remainder =
                    divide_to_writeable(&mut string, i.0, i.1, PRECISION, false).unwrap();

                assert_eq!(string, i.2);
                assert_eq!(remainder.is_zero(), i.3);
            }
        }

        #[cfg(feature = "with-bigint")]
        {
            for i in data {
                assert_eq!(
                    divide_to_string(BigUint::from(i.0), BigUint::from(i.1), PRECISION, false)
                        .unwrap(),
                    i.2
                );
            }
        }
    }

    #[test]
    fn test_divide_to_string() {
        assert_eq!(divide_to_string(0, 5, 5, false).unwrap(), "0");
        assert_eq!(divide_to_string(0, 5, 5, true).unwrap(), "0.00000");
        assert_eq!(divide_to_string(30, 2, 5, false).unwrap(), "15");
        assert_eq!(divide_to_string(30, 2, 5, true).unwrap(), "15.00000");
        assert_eq!(divide_to_string(2, 4, 2, false).unwrap(), "0.5");
        assert_eq!(divide_to_string(2, 4, 2, true).unwrap(), "0.50");
        assert_eq!(divide_to_string(255u8, 3u8, 5, false).unwrap(), "85");
        assert_eq!(divide_to_string(255u8, 3u8, 5, true).unwrap(), "85.00000");

        assert_eq!(divide_to_string(155u8, 253u8, 5, false).unwrap(), "0.61264");
        assert_eq!(divide_to_string(1u8, 2u8, 1, false).unwrap(), "0.5");
        assert_eq!(divide_to_string(1u8, 2u8, 1, true).unwrap(), "0.5");

        assert_eq!(
            divide_to_string(1, 3, 28, false).unwrap(),
            "0.3333333333333333333333333333"
        );
        assert_eq!(
            divide_to_string(1, 3, 28, true).unwrap(),
            "0.3333333333333333333333333333"
        );
        assert_eq!(divide_to_string(806, 31, 0, false).unwrap(), "26");
        assert_eq!(divide_to_string(806, 31, 0, true).unwrap(), "26");
        assert_eq!(divide_to_string(807, 31, 4, false).unwrap(), "26.0322");
        assert_eq!(divide_to_string(807, 31, 4, true).unwrap(), "26.0322");

        if let Err(DivisionError::DivisionByZero) = divide_to_string(1, 0, 1, false) {
        } else {
            assert!(false);
        };

        if let Err(DivisionError::DivisionByZero) = divide_to_string(1, 0, 1, true) {
        } else {
            assert!(false);
        };

        assert_eq!(divide_to_string(1, 10000, 2, false).unwrap(), "0");
        assert_eq!(divide_to_string(1, 10000, 2, true).unwrap(), "0.00");

        #[cfg(feature = "with-bigint")]
        {
            let num = "820123456789012345678901234567890123456789";
            let den = "420420420420240240420240420240420420420";
            let result = "1950.7222222204153880534352079149419688493160244330759069204925259490806291881834407646199555744655166719234903156227406500953778757619920899563294795746797267246703798051247915744867884912121330007705286911209791422213223230426822190127666529437572025264829201539629594175021000386486667365851813562015885600393107707737453721144405603009059457352854387023006301508907770762997683254372534811586343569328131455924964081595076622200116354403760742833073621862325616259444084808295834752749082040590201012701034118255745516514972270054835928011374231619434684843847682844123336846713100440285930668493675706096464476187809105398269815519780303174023949885603320678109566772031394249633001137109155600514761384776616827086908396960584246277151426685095767130738996420461009015758184659365258827537226581854494195009714400547427050697946126012192691851549545189173091941689299722345297086508711845865506663262915436874050594522308464492248968916100678819937355384703664061966410613989688966690413319849627099468906113991173042101218";

            let asrt1 = divide_to_string(
                BigUint::from_str_radix(num, 10).ok().unwrap(),
                BigUint::from_str_radix(den, 10).ok().unwrap(),
                1024,
                false,
            );

            assert_eq!(&asrt1.ok().unwrap(), result);

            let asrt1 = divide_to_string(
                BigUint::from_str_radix(num, 10).ok().unwrap(),
                BigUint::from_str_radix(den, 10).ok().unwrap(),
                1024,
                true,
            );

            assert_eq!(&asrt1.ok().unwrap(), result);
        }
    }

    #[test]
    fn test_divide_to_ascii_vec() {
        assert_eq!(divide_to_ascii_vec(0, 5, 5, false).unwrap(), vec![48]);
        assert_eq!(
            divide_to_ascii_vec(0, 5, 5, true).unwrap(),
            vec![48, 46, 48, 48, 48, 48, 48]
        );
        assert_eq!(divide_to_ascii_vec(30, 2, 5, false).unwrap(), vec![49, 53]);
        assert_eq!(
            divide_to_ascii_vec(2, 4, 2, false).unwrap(),
            vec![48, 46, 53]
        );
        assert_eq!(
            divide_to_ascii_vec(2, 4, 2, true).unwrap(),
            vec![48, 46, 53, 48]
        );
        assert_eq!(
            divide_to_ascii_vec(255u8, 3u8, 5, false).unwrap(),
            vec![56, 53]
        );
        assert_eq!(
            divide_to_ascii_vec(1000001u64, 10000u64, 3, false).unwrap(),
            vec![49, 48, 48]
        );
        assert_eq!(
            divide_to_ascii_vec(1000001u64, 10000u64, 3, true).unwrap(),
            vec![49, 48, 48, 46, 48, 48, 48]
        );
    }

    #[test]
    fn test_divide_integral() {
        {
            let mut r1: [u8; 1] = [0; 1];
            let mut p1: usize = 0;

            let mut r2: [u8; 2] = [0; 2];
            let mut p2: usize = 0;

            let mut r3: [u8; 3] = [0; 3];
            let mut p3: usize = 0;

            let rest1 = divide_integral(2, 4, |d| {
                r1[p1] = d;
                p1 += 1;
                Ok(true)
            });

            assert_eq!(r1, [0]);
            assert_eq!(p1, 1);
            assert_eq!(rest1.unwrap(), DivisionState::new(2, 4));

            let rest2 = divide_integral(82, 3, |d| {
                r2[p2] = d;
                p2 += 1;
                Ok(true)
            });

            assert_eq!(r2, [2, 7]);
            assert_eq!(p2, 2);
            assert_eq!(rest2.unwrap(), DivisionState::new(1, 3));

            let rest3 = divide_integral(2020, 4, |d| {
                r3[p3] = d;
                p3 += 1;
                Ok(true)
            });

            assert_eq!(r3, [5, 0, 5]);
            assert_eq!(p3, 3);
            assert_eq!(rest3.unwrap(), DivisionState::new(0, 4));
        }

        {
            let mut result = Vec::new();
            let rem = divide_integral(255u8, 3u8, |d| {
                result.push(d);
                Ok(true)
            });

            assert_eq!(rem.ok(), Some(DivisionState::new(0, 3)));
            assert_eq!(result, vec![8, 5]);
        }

        {
            let mut result = Vec::new();
            let rem = divide_integral(255u8, 3u8, |d| {
                result.push(d);
                Ok(false)
            });

            assert_eq!(rem.ok(), Some(DivisionState::new(15, 3)));
            assert_eq!(result, vec![8]);
        }
    }

    #[test]
    fn test_divide_rem() {
        let mut r1: [u8; 1] = [0; 1];
        let mut p1: usize = 0;

        let mut r2: [u8; 2] = [0; 2];
        let mut p2: usize = 0;

        let mut r3: [u8; 3] = [0; 3];
        let mut p3: usize = 0;

        let mut rest1 = None;
        divide_rem(1, 3, |s, d| {
            r1[p1] = d;
            p1 += 1;
            rest1 = Some(s.remainder);
            Ok(Err(s))
        })
        .ok();

        assert_eq!(r1, [3]);
        assert_eq!(p1, 1);
        assert_eq!(rest1, Some(1));

        p1 = 0;
        let rest1 = divide_rem(1, 2, |s, d| {
            r1[p1] = d;
            p1 += 1;
            Ok(Ok(s))
        });

        assert_eq!(r1, [5]);
        assert_eq!(p1, 1);
        assert_eq!(rest1.unwrap(), DivisionState::new(0, 2));

        let rest2 = divide_rem(500, 2000, |s, d| {
            r2[p2] = d;
            p2 += 1;
            Ok(Ok(s))
        });

        assert_eq!(r2, [2, 5]);
        assert_eq!(p2, 2);
        assert_eq!(rest2.unwrap(), DivisionState::new(0, 2000));

        let rest3 = divide_rem(2, 1000, |s, d| {
            r3[p3] = d;
            p3 += 1;
            Ok(Ok(s))
        });

        assert_eq!(r3, [0, 0, 2]);
        assert_eq!(p3, 3);
        assert_eq!(rest3.unwrap(), DivisionState::new(0, 1000));

        p3 = 0;
        let rest3 = divide_rem(502, 1000, |s, d| {
            r3[p3] = d;
            p3 += 1;
            Ok(Ok(s))
        });

        assert_eq!(r3, [5, 0, 2]);
        assert_eq!(p3, 3);
        assert_eq!(rest3.unwrap(), DivisionState::new(0, 1000));
    }
}
