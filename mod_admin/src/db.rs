use rustbot::prelude::*;

use postgres::types::FromSql;

pub fn query(ctx: &Context, args: &str) -> Result<()> {
    let result: String = {
        let db = ctx.bot.sql().lock();
        let r = db.prepare(args).and_then(|stmt| {
            if stmt.columns().len() == 0 {
                db.execute(args, &[]).map(|n| format!("{} row(s) changed", n))
            } else {
                let cols: Vec<String> = stmt.columns().iter().map(|s| format!("{} {}", s.name(), s.type_().name())).collect();
                let colstr = format!("({})", cols.join(", "));
                let row_strs: Vec<String> = stmt
                    .query(&[])?
                    .iter()
                    .map(|row| {
                        let vals: Vec<String> = (0..row.len())
                            .map(|i| {
                                let ty = row.columns()[i].type_();
                                if row.get_bytes(i).is_none() {
                                    format!("null")
                                } else if i8::accepts(ty) {
                                    format!("{}", row.get::<_, i8>(i))
                                } else if i16::accepts(ty) {
                                    format!("{}", row.get::<_, i16>(i))
                                } else if i32::accepts(ty) {
                                    format!("{}", row.get::<_, i32>(i))
                                } else if i64::accepts(ty) {
                                    format!("{}", row.get::<_, i64>(i))
                                } else if f32::accepts(ty) {
                                    format!("{}", row.get::<_, f32>(i))
                                } else if f64::accepts(ty) {
                                    format!("{}", row.get::<_, f64>(i))
                                } else if String::accepts(ty) {
                                    format!("{:?}", row.get::<_, String>(i))
                                } else if bool::accepts(ty) {
                                    format!("{:?}", row.get::<_, bool>(i))
                                } else if serde_json::Value::accepts(ty) {
                                    format!("{}", row.get::<_, serde_json::Value>(i))
                                } else {
                                    format!("<type={}>", ty.name())
                                }
                            })
                            .collect();
                        format!("({})", vals.join(", "))
                    })
                    .collect();
                Ok(format!("{}: {}", colstr, row_strs.join(", ")))
            }
        });
        r?
    };
    ctx.say(result.as_str())
}
