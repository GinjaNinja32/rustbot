use rand::{rngs::StdRng, SeedableRng};
use std::collections::BTreeMap;

use rustbot::prelude::{spans, spans_to_raw_string};

use super::ast::*;
use super::limits::Limiter;
use super::value::Value;

macro_rules! test_parser {
    ($parser:ident:
        $( $input:literal => $output:pat $(if $cond:expr)?,)* ) => {
        $(
            {
                let res = $parser::parse($input).unwrap();
                if res.0 != "" {
                    panic!("did not consume all input text\ninput: {:?}\ngot: {:?}\nleftover text: {:?}",
                        $input,
                        res.1,
                        res.0,
                    );
                }
                if let ("", $output) = &res {
                    if !(true $( && $cond )?) {
                        panic!("result condition failed\ninput: {:?}\nexpect: {}\ncondition: {}\ngot: {:?}",
                            $input,
                            stringify!($output),
                            stringify!($($cond)?),
                            res.1,
                        );
                    }
                } else {
                    panic!("result pattern did not match\ninput: {:?}\nexpect: {}\ngot: {:?}",
                        $input,
                        stringify!($output),
                        res.1,
                    );
                }
            }
        )*
    }
}
macro_rules! test_evaluation {
    ($node:ident:
        $( $input:literal $(where { $( $name:literal: $val:expr ),* })? => $value:expr,)*) => {
        $(
            {
                let ast = match $node::parse($input).unwrap() {
                    ("", ast) => ast,
                    (leftover, ast) => {
                        panic!("did not consume all input text\ninput: {:?}\ngot: {:?}\nleftover text: {:?}",
                            $input,
                            ast,
                            leftover,
                        )
                    }
                };

                #[allow(unused_mut)]
                let mut values = BTreeMap::new();
                $($(
                    values.insert($name, (spans!{stringify!($name)}, $val));
                )*)?


                let mut ctx = EvalContext {
                    limit: &mut Limiter::new(100),
                    rng: &mut StdRng::seed_from_u64(0),
                    values,
                };

                let value = ast.eval(&mut ctx).unwrap().1;
                if value != $value {
                    panic!("result did not match expectation\ninput: {:?}\nexpect: {:?}\ngot: {:?}",
                        $input,
                        $value,
                        value,
                    );
                }
            }
        )*
    }
}

macro_rules! test_command_eval {
    ($( $input:literal => $result:literal,)*) => {
        $(
            {
                let mut limit = Limiter::new(100);
                let ast = match Command::parse($input).unwrap() {
                    ("", ast) => ast,
                    (leftover, ast) => {
                        panic!("did not consume all input text\ninput: {:?}\ngot: {:?}\nleftover text: {:?}",
                            $input,
                            ast,
                            leftover,
                        )
                    }
                };

                let mut rng = StdRng::seed_from_u64(0);

                let spans = ast.eval(&mut limit, &mut rng).unwrap();
                let raw_output = spans_to_raw_string(spans);
                if raw_output != $result {
                    panic!("result did not match expectation\ninput: {:?}\nexpect: {:?}\ngot:    {:?}",
                        $input,
                        $result,
                        raw_output,
                    );
                }
            }
        )*
    }
}

#[test]
fn test_command() {
    test_parser!(
    Command:
        "2d6" => Command{ bindings, output }
            if bindings.0.is_empty()
            && matches!(output, CommandResult::Simple(_)),

        "A: 42; B: $Ad6;; $A and $B" => Command{ bindings, output }
            if matches!(bindings.0.as_slice(), [('A', _), ('B', _)])
            && matches!(output, CommandResult::Complex(segments)
                if matches!(segments.as_slice(), [OutputSegment::Value('A'), OutputSegment::Text(and), OutputSegment::Value('B')] if and == " and ")
            ),
    );

    test_command_eval!(
        "2d6" => "[3, 6]: 2d6:[3, 6]",
        "6#s4d6" => "[11, 17, 10, 17, 10, 12]: s4d6:[3, 6, 1, 1], s4d6:[5, 3, 5, 4], s4d6:[2, 2, 3, 3], s4d6:[3, 6, 6, 2], s4d6:[5, 1, 1, 3], s4d6:[4, 2, 3, 3]",
        "R: 2d6; $R" => "[3, 6]: 2d6:[3, 6]",
        "R: 2d6;; $R" => "[3, 6]",

        // '!space 1' through '!space 6'
        "D:1; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3] (1): 0 successes, 0 sixes, 0 ones",
        "D:2; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3, 6] (2): 1 success, 1 six - crit, 0 ones",
        "D:3; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3, 6, 1] (3): 1 success, 1 six, 1 one",
        "D:4; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3, 6, 1, 1] (4): 1 success, 1 six, 2 ones - critfail",
        "D:5; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3, 6, 1, 1, 4] (5): 1 success, 1 six, 2 ones",
        "D:6; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); T:($D+1)/2; c:$C>=$T; o:$O>=$T;; $R ($D): $S success%[es], $C six%[es]%$c[| - crit], $O one%s%$o[| - critfail]"
            => "[3, 6, 1, 1, 4, 5] (6): 2 successes, 1 six, 2 ones",
    );
}

#[test]
fn test_expression() {
    test_parser!(
    Expression:
        "2d6" => Expression{..},
        " 2 d 6 " => Expression{..},
    );

    test_evaluation!(
    Expression:
        "2d6" => Value::IntSlice(vec![3, 6]),
        "6#s4d6" => Value::IntSlice(vec![11, 17, 10, 17, 10, 12]),
    );
}

#[test]
fn test_repeat() {
    test_parser!(
    Repeat:
        "2d6" => Repeat{repeat: None, ..},
        "2#2d6" => Repeat{repeat: Some(2), ..},
        "2 # 2 d 6" => Repeat{repeat: Some(2), ..},
    );

    test_evaluation!(
    Repeat:
        "2d6" => Value::IntSlice(vec![3, 6]),
        "2#2d6" => Value::IntSlice(vec![9, 5]),
        "6#2d6" => Value::IntSlice(vec![9, 5, 8, 7, 6, 9]),
    );
}

#[test]
fn test_comparison() {
    test_parser!(
    Comparison:
        "2 d 6 > 4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Greater, each_right: false}, _))},
        "2 d 6 e>e 4" => Comparison{left: _, right: Some((CompareOp{each_left: true,  op: CompareBaseOp::Greater, each_right: true},  _))},

        "2d6>4"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Greater,   each_right: false}, _))},
        "2d6<4"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Less,      each_right: false}, _))},
        "2d6>=4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::GreaterEq, each_right: false}, _))},
        "2d6=>4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::GreaterEq, each_right: false}, _))},
        "2d6<=4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::LessEq,    each_right: false}, _))},
        "2d6=<4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::LessEq,    each_right: false}, _))},
        "2d6==4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Equal,     each_right: false}, _))},
        "2d6=4"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Equal,     each_right: false}, _))},
        "(2d6)!=4" => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Unequal,   each_right: false}, _))}, // brackets to avoid `(2d6!) = 4` interpretation
        "2d6<>4"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Unequal,   each_right: false}, _))},

        "2d6e>4"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Greater,   each_right: false}, _))},
        "2d6e<4"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Less,      each_right: false}, _))},
        "2d6e>=4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::GreaterEq, each_right: false}, _))},
        "2d6e=>4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::GreaterEq, each_right: false}, _))},
        "2d6e<=4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::LessEq,    each_right: false}, _))},
        "2d6e=<4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::LessEq,    each_right: false}, _))},
        "2d6e==4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Equal,     each_right: false}, _))},
        "2d6e=4"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Equal,     each_right: false}, _))},
        "2d6e!=4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Unequal,   each_right: false}, _))},
        "2d6e<>4"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Unequal,   each_right: false}, _))},

        "2d6>e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Greater,   each_right: true}, _))},
        "2d6<e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Less,      each_right: true}, _))},
        "2d6>=e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::GreaterEq, each_right: true}, _))},
        "2d6=>e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::GreaterEq, each_right: true}, _))},
        "2d6<=e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::LessEq,    each_right: true}, _))},
        "2d6=<e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::LessEq,    each_right: true}, _))},
        "2d6==e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Equal,     each_right: true}, _))},
        "2d6=e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Equal,     each_right: true}, _))},
        "(2d6)!=e[3,4]" => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Unequal,   each_right: true}, _))}, // brackets to avoid `(2d6!) =e [3,4]` interpretation
        "2d6<>e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: false, op: CompareBaseOp::Unequal,   each_right: true}, _))},

        "2d6e>e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Greater,   each_right: true}, _))},
        "2d6e<e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Less,      each_right: true}, _))},
        "2d6e>=e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::GreaterEq, each_right: true}, _))},
        "2d6e=>e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::GreaterEq, each_right: true}, _))},
        "2d6e<=e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::LessEq,    each_right: true}, _))},
        "2d6e=<e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::LessEq,    each_right: true}, _))},
        "2d6e==e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Equal,     each_right: true}, _))},
        "2d6e=e[3,4]"    => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Equal,     each_right: true}, _))},
        "2d6e!=e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Unequal,   each_right: true}, _))},
        "2d6e<>e[3,4]"   => Comparison{left: _, right: Some((CompareOp{each_left: true, op: CompareBaseOp::Unequal,   each_right: true}, _))},
    );

    test_evaluation!(
    Comparison:
        "2d6" => Value::IntSlice(vec![3, 6]),

        "2d6=8" => Value::Bool(false),
        "2d6=9" => Value::Bool(true),

        "2d6<>8" => Value::Bool(true),
        "2d6<>9" => Value::Bool(false),

        "2d6>8" => Value::Bool(true),
        "2d6>9" => Value::Bool(false),
        "2d6>=9" => Value::Bool(true),
        "2d6>=10" => Value::Bool(false),

        "2d6<10" => Value::Bool(true),
        "2d6<9" => Value::Bool(false),
        "2d6<=9" => Value::Bool(true),
        "2d6<=8" => Value::Bool(false),

        "2d6e=3" => Value::BoolSlice(vec![true, false]),
        "2d6e<>3" => Value::BoolSlice(vec![false, true]),

        "2d6e=e[3,6]" => Value::BoolSlice(vec![true, true]),
    );
}

#[test]
fn test_addsub() {
    test_parser!(
    AddSub:
        "2+2"   => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Add, each_right: false}, _)]),
        "2-2"   => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Sub, each_right: false}, _)]),
        "2+2-2" => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Add, each_right: false}, _), (AddSubOp{each_left: false, op: AddSubBaseOp::Sub, each_right: false}, _)]),
        "2-2+2" => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Sub, each_right: false}, _), (AddSubOp{each_left: false, op: AddSubBaseOp::Add, each_right: false}, _)]),

        "2 - 2 + 2" => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Sub, each_right: false}, _), (AddSubOp{each_left: false, op: AddSubBaseOp::Add, each_right: false}, _)]),

        "2e+2"    => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: true,  op: AddSubBaseOp::Add, each_right: false}, _)]),
        "2e-2"    => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: true,  op: AddSubBaseOp::Sub, each_right: false}, _)]),
        "2+e2"    => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Add, each_right: true},  _)]),
        "2-e2"    => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: false, op: AddSubBaseOp::Sub, each_right: true},  _)]),
        "2e+e2"   => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: true,  op: AddSubBaseOp::Add, each_right: true},  _)]),
        "2e-e2"   => AddSub{left: _, right} if matches!(right.as_slice(), [(AddSubOp{each_left: true,  op: AddSubBaseOp::Sub, each_right: true},  _)]),
    );

    test_evaluation!(
    AddSub:
        "2+2" => Value::Int(4),
        "2-2" => Value::Int(0),
        "5+3-2" => Value::Int(6),
        "5-2+3" => Value::Int(6),

        "[1,2,3] e+ 2" => Value::IntSlice(vec![3,4,5]),
        "[1,2,3] e- 2" => Value::IntSlice(vec![-1,0,1]),

        "10 +e [1,2,3]" => Value::IntSlice(vec![11,12,13]),
        "10 -e [1,2,3]" => Value::IntSlice(vec![9,8,7]),

        "[5,7,11] e+e [1,2,3]" => Value::IntSlice(vec![6,9,14]),
        "[5,7,11] e-e [1,2,3]" => Value::IntSlice(vec![4,5,8]),
    );
}

#[test]
fn test_muldiv() {
    test_parser!(
    MulDiv:
        "2*2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),
        "2/2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2*2/2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2/2*2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),

        "2 / 2 * 2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),

        "[2]e*2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Mul, each_right: false}, _)]),
        "[2]e/2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2*e[2]"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: true},  _)]),
        "2/e[2]"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: true},  _)]),
        "[2]e*e[2]" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Mul, each_right: true},  _)]),
        "[2]e/e[2]" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Div, each_right: true},  _)]),
    );

    test_evaluation!(
    MulDiv:
        "2*2" => Value::Int(4),
        "2/2" => Value::Int(1),
        "5*3/2" => Value::Int(7),
        "5/2*3" => Value::Int(6),

        "[1,2,3] e* 2" => Value::IntSlice(vec![2, 4, 6]),
        "[1,2,3] e/ 2" => Value::IntSlice(vec![0, 1, 1]),

        "10 *e [1,2,3]" => Value::IntSlice(vec![10,20,30]),
        "10 /e [1,2,3]" => Value::IntSlice(vec![10,5,3]),

        "[5,7,11] e*e [1,2,3]" => Value::IntSlice(vec![5,14,33]),
        "[5,7,11] e/e [1,2,3]" => Value::IntSlice(vec![5,3,3]),
    );
}

#[test]
fn test_sum() {
    test_parser!(
    Sum:
        "2d6" => Sum{is_sum: false, ..},
        "s2d6" => Sum{is_sum: true, ..},
        "s 2 d 6" => Sum{is_sum: true, ..},
    );

    test_evaluation!(
    Sum:
        "4d6" => Value::IntSlice(vec![3, 6, 1, 1]),
        "s4d6" => Value::Int(11),
    );
}

#[test]
fn test_dicemod() {
    test_parser!(
    DiceMod:
        "4d6" => DiceMod{op: None, ..},
        "4d6H3" => DiceMod{op: Some((ModOp::KeepHighest, _)), ..},
        "4d6h3" => DiceMod{op: Some((ModOp::DropHighest, _)), ..},
        "4d6L1" => DiceMod{op: Some((ModOp::KeepLowest, _)), ..},
        "4d6l1" => DiceMod{op: Some((ModOp::DropLowest, _)), ..},

        "4 d 6 H 3" => DiceMod{op: Some((ModOp::KeepHighest, _)), ..},
    );

    test_evaluation!(
    DiceMod:
        "4d6"   => Value::IntSlice(vec![3, 6, 1, 1]),

        "4d6h3" => Value::IntSlice(vec![1]),
        "4d6h2" => Value::IntSlice(vec![1, 1]),
        "4d6h1" => Value::IntSlice(vec![1, 1, 3]),
        "4d6H3" => Value::IntSlice(vec![   1, 3, 6]),
        "4d6H2" => Value::IntSlice(vec![      3, 6]),
        "4d6H1" => Value::IntSlice(vec![         6]),

        "4d6l3" => Value::IntSlice(vec![         6]),
        "4d6l2" => Value::IntSlice(vec![      3, 6]),
        "4d6l1" => Value::IntSlice(vec![   1, 3, 6]),
        "4d6L3" => Value::IntSlice(vec![1, 1, 3]),
        "4d6L2" => Value::IntSlice(vec![1, 1]),
        "4d6L1" => Value::IntSlice(vec![1]),
    );
}

#[test]
fn test_diceroll() {
    test_parser!(
    DiceRoll:
        "42" => DiceRoll::NoRoll(_),
        "[1,2,3] @ 2" => DiceRoll::Index{each: false, ..},
        "[1,2,3] @e [1,2]" => DiceRoll::Index{each: true, ..},
        "d"      => DiceRoll::Roll{count: None,    sides: None,    explode: None},
        "d!"     => DiceRoll::Roll{count: None,    sides: None,    explode: Some(Explode::Default)},
        "d!5"    => DiceRoll::Roll{count: None,    sides: None,    explode: Some(Explode::Target(5))},
        "d20"    => DiceRoll::Roll{count: None,    sides: Some(_), explode: None},
        "d20!"   => DiceRoll::Roll{count: None,    sides: Some(_), explode: Some(Explode::Default)},
        "d20!19" => DiceRoll::Roll{count: None,    sides: Some(_), explode: Some(Explode::Target(19))},
        "3d"     => DiceRoll::Roll{count: Some(_), sides: None,    explode: None},
        "3d!"    => DiceRoll::Roll{count: Some(_), sides: None,    explode: Some(Explode::Default)},
        "3d!5"   => DiceRoll::Roll{count: Some(_), sides: None,    explode: Some(Explode::Target(5))},
        "2d6"    => DiceRoll::Roll{count: Some(_), sides: Some(_), explode: None},
        "2d6!"   => DiceRoll::Roll{count: Some(_), sides: Some(_), explode: Some(Explode::Default)},
        "2d6!5"  => DiceRoll::Roll{count: Some(_), sides: Some(_), explode: Some(Explode::Target(5))},
    );

    test_evaluation!(
    DiceRoll:
        "42" => Value::Int(42),
        "[1,2,3] @ 2" => Value::Int(3),
        "[1,2,3] @e [1,2]" => Value::IntSlice(vec![2, 3]),
        "d"     => Value::IntSlice(vec![3]),
        "5d"    => Value::IntSlice(vec![3, 6, 1, 1, 4]),
        "d!2"   => Value::IntSlice(vec![3, 6, 1]),
        "d20"   => Value::IntSlice(vec![19]),
        "5d20"  => Value::IntSlice(vec![19, 7, 1, 1, 12]),
        "d20!5" => Value::IntSlice(vec![19, 7, 1]),
        "10d[0,1]" => Value::IntSlice(vec![0, 0, 1, 1, 1, 1, 0, 0, 0, 1]),
    );
}

#[test]
fn test_value() {
    test_parser!(
    AstValue:
        "6" => AstValue::Int(6),
        "(42)" => AstValue::Sub(_),
        "[1,2,3]" => AstValue::Slice(slice) if slice.len() == 3,
        "F" => AstValue::Fate,
        "%" => AstValue::Hundred,
        "$A" => AstValue::Binding('A'),
    );

    test_evaluation!(
    AstValue:
        "6" => Value::Int(6),
        "(42)" => Value::Int(42),
        "[1,2,3]" => Value::IntSlice(vec![1, 2, 3]),
        "F" => Value::IntSlice(vec![-1,0,1]),
        "%" => Value::Int(100),
        "$A" where {'A': Value::Int(42)} => Value::Int(42),
    );
}
