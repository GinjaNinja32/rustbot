use rusqlite::types::ValueRef::*;
use rusqlite::NO_PARAMS;
use rustbot::prelude::*;

pub fn query(ctx: &Context, args: &str) -> Result<()> {
    let result: String = {
        let db = ctx.bot.sql().lock();
        let r = db.prepare(args).and_then(|mut stmt| {
            if stmt.column_count() == 0 {
                db.execute(args, NO_PARAMS).map(|n| format!("{} row(s) changed", n))
            } else {
                let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                let colstr = format!("({})", cols.join(", "));
                stmt.query_map(NO_PARAMS, |row| {
                    let vals: Vec<String> = (0..row.column_count())
                        .map(|i| match row.get_raw(i) {
                            Null => "null".to_string(),
                            Integer(i) => format!("{}", i),
                            Real(f) => format!("{}", f),
                            Text(s) => format!("{:?}", s),
                            Blob(b) => format!("{:?}", b),
                        })
                        .collect();
                    format!("({})", vals.join(", "))
                })
                .and_then(|rows| {
                    let r: std::result::Result<Vec<String>, rusqlite::Error> = rows.collect();
                    Ok(format!("{}: {}", colstr, r?.join(", ")))
                })
            }
        });
        r?
    };
    ctx.say(result.as_str())
}
