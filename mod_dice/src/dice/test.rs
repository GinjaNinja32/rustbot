use super::ast::*;

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

#[test]
fn test_parse_command() {
    test_parser!(
    Command:
        "2d6" => Command::Simple(_),
        "A: 42; B: $Ad6; $A and $B" => Command::Complex{ bindings, output }
            if matches!(bindings.as_slice(), [('A', _), ('B', _)])
            && matches!(output.as_slice(), [OutputSegment::Value('A'), OutputSegment::Text(and), OutputSegment::Value('B')] if and == " and "),
    );
}

#[test]
fn test_parse_expression() {
    test_parser!(
    Expression:
        "2d6" => Expression{..},
        " 2 d 6 " => Expression{..},
    );
}

#[test]
fn test_parse_repeat() {
    test_parser!(
    Repeat:
        "2d6" => Repeat{repeat: None, ..},
        "2#2d6" => Repeat{repeat: Some(2), ..},
        "2 # 2 d 6" => Repeat{repeat: Some(2), ..},
    );
}

#[test]
fn test_parse_comparison() {
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
}

#[test]
fn test_parse_addsub() {
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
}

#[test]
fn test_parse_muldiv() {
    test_parser!(
    MulDiv:
        "2*2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),
        "2/2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2*2/2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2/2*2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),

        "2 / 2 * 2" => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: false}, _), (MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: false}, _)]),

        "2e*2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Mul, each_right: false}, _)]),
        "2e/2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Div, each_right: false}, _)]),
        "2*e2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Mul, each_right: true},  _)]),
        "2/e2"    => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: false, op: MulDivBaseOp::Div, each_right: true},  _)]),
        "2e*e2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Mul, each_right: true},  _)]),
        "2e/e2"   => MulDiv{left: _, right} if matches!(right.as_slice(), [(MulDivOp{each_left: true,  op: MulDivBaseOp::Div, each_right: true},  _)]),
    );
}

#[test]
fn test_parse_sum() {
    test_parser!(
    Sum:
        "2d6" => Sum{is_sum: false, ..},
        "s2d6" => Sum{is_sum: true, ..},
        "s 2 d 6" => Sum{is_sum: true, ..},
    );
}

#[test]
fn test_parse_dicemod() {
    test_parser!(
    DiceMod:
        "4d6" => DiceMod{op: None, ..},
        "4d6H3" => DiceMod{op: Some((ModOp::KeepHighest, _)), ..},
        "4d6h3" => DiceMod{op: Some((ModOp::DropHighest, _)), ..},
        "4d6L1" => DiceMod{op: Some((ModOp::KeepLowest, _)), ..},
        "4d6l1" => DiceMod{op: Some((ModOp::DropLowest, _)), ..},

        "4 d 6 H 3" => DiceMod{op: Some((ModOp::KeepHighest, _)), ..},
    );
}

#[test]
fn test_parse_diceroll() {
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
}

#[test]
fn test_parse_value() {
    test_parser!(
    AstValue:
        "6" => AstValue::Int(6),
        "(42)" => AstValue::Sub(_),
        "[1,2,3]" => AstValue::Slice(slice) if slice.len() == 3,
        "F" => AstValue::Fate,
        "%" => AstValue::Hundred,
        "$A" => AstValue::Binding('A'),
    );
}
