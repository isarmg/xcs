#[cfg(test)]
mod tests {
    use crate::ascii::digit1;
    use crate::combinator::fail;
    use crate::dispatch;
    use crate::error::ContextError;
    use crate::token::any;

    use super::*;

    fn parser<'i>() -> impl Parser<&'i str, i32, ContextError> {
        move |i: &mut &str| {
            use Infix::*;
            expression(digit1.parse_to::<i32>())
                .current_precedence_level(0)
                .prefix(dispatch! {any;
                    '+' => Prefix(12, |_, a| Ok(a)),
                    '-' => Prefix(12, |_, a: i32| Ok(-a)),
                    _ => fail
                })
                .infix(dispatch! {any;
                   '+' => Left(5, |_, a, b| Ok(a + b)),
                   '-' => Left(5, |_, a, b| Ok(a - b)),
                   '*' => Left(7, |_, a, b| Ok(a * b)),
                   '/' => Left(7, |_, a, b| Ok(a / b)),
                   '%' => Left(7, |_, a, b| Ok(a % b)),
                   '^' => Left(9, |_, a, b| Ok(a ^ b)),
                   _ => fail
                })
                .parse_next(i)
        }
    }

    #[test]
    fn test_expression() {
        assert_eq!(parser().parse("-3+-3*4"), Ok(-15));
        assert_eq!(parser().parse("+2+3*4"), Ok(14));
        assert_eq!(parser().parse("2*3+4"), Ok(10));
    }
}
