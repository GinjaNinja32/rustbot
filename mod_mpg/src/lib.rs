use rustbot::prelude::*;

#[cfg(test)]
mod tests;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("mpg", Command::new(mpg).req_perms(Perms::Admin));
}

struct MpgEntry {
    mileage: i32,
    fill_litres: f64,
    fill_price: f64,
    result_price: Option<f64>,
}

const TANK_SIZE: f64 = 65.0;

fn load_entries(ctx: &dyn Context) -> Result<Vec<MpgEntry>> {
    let res = ctx.bot().sql().lock().query(
        "SELECT mileage, fill_litres, fill_price, result_price
        FROM mpg
        WHERE mileage >= (SELECT max(mileage) FROM mpg WHERE result_price IS NOT NULL)
        ORDER BY mileage DESC",
        &[],
    )?;

    let mut entries = vec![];

    for row in res {
        entries.push(MpgEntry {
            mileage: row.get(0),
            fill_litres: row.get(1),
            fill_price: row.get(2),
            result_price: row.get(3),
        })
    }

    Ok(entries)
}

fn mpg(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(4, ' ').collect();
    let (mileage, litres, price, full) = match args[..] {
        [m, l, p] => (m, l, p, false),
        [m, l, p, "full"] => (m, l, p, true),
        _ => bail_user!("usage: mpg <mileage> <litres> <price> [\"full\"]"),
    };

    let mileage = mileage
        .parse::<i32>()
        .map_err(|_| UserError::new("mileage must be an int"))?;
    let litres = litres
        .parse::<f64>()
        .map_err(|_| UserError::new("litres must be a float"))?;
    let price = price
        .parse::<f64>()
        .map_err(|_| UserError::new("price must be a float"))?;

    if !full {
        ctx.bot().sql().lock().query(
            "INSERT INTO mpg (mileage, fill_litres, fill_price, result_price)
            VALUES ($1, $2, $3, NULL)",
            &[&mileage, &litres, &price],
        )?;
        return ctx.reply(Message::Simple("Data recorded".into()));
    }

    let entries = load_entries(ctx)?;

    let MpgStats {
        total_mileage,
        used_litres,
        used_cost,
        mpg,
        mpg_us,
        lp100km,
        perlitre,
        permile,
        perkm,
        result_price,
    } = calculate_stats(mileage, litres, price, &entries);

    ctx.reply(Message::Simple(format!(
        "since last fill: {total_mileage} miles, {used_litres:.2} litres, £{used_cost:.2}\nstats: {mpg:.2} mpg, {mpg_us:.2} mpg(US), {lp100km:.2} l/100km\navg cost: £{perlitre:.3}/litre, £{permile:.3}/mile, £{perkm:.3}/km"
    )))?;

    ctx.bot().sql().lock().query(
        "INSERT INTO mpg (mileage, fill_litres, fill_price, result_price)
        VALUES ($1, $2, $3, $4)",
        &[&mileage, &litres, &price, &result_price],
    )?;
    return ctx.reply(Message::Simple("Data recorded".into()));
}

#[derive(Debug, PartialEq)]
struct MpgStats {
    total_mileage: i32,
    used_litres: f64,
    used_cost: f64,

    mpg: f64,
    mpg_us: f64,
    lp100km: f64,

    perlitre: f64,
    permile: f64,
    perkm: f64,

    result_price: f64,
}

fn calculate_stats(end_mileage: i32, fill_litres: f64, fill_price: f64, entries: &[MpgEntry]) -> MpgStats {
    let mut used_litres = fill_litres;
    let mut total_litres = 0f64;
    let mut start_mileage = 0;
    let mut start_price = 0f64;

    for entry in entries {
        let e_litres = if let Some(rp) = entry.result_price {
            // Remainder of tank is this price.
            let remainder_litres = TANK_SIZE - total_litres;
            start_mileage = entry.mileage;
            start_price = rp;
            remainder_litres
        } else {
            // Partial fill.
            used_litres += entry.fill_litres;
            entry.fill_litres
        };

        total_litres += e_litres;
    }

    if start_mileage == 0 {
        panic!("start mileage was 0?");
    }

    let total_mileage = end_mileage - start_mileage;

    let mpl = total_mileage as f64 / used_litres;
    let mpg = mpl * 4.54609;
    let mpg_us = mpl * 3.7854118;
    let lp100km = 62.137119 / mpl;

    let mut tank_price_at = start_mileage;
    let mut tank_price = start_price;
    let mut tank_amount = TANK_SIZE;

    let mut used_cost = 0f64;

    for entry in entries.iter().rev().skip(1).chain(std::iter::once(&MpgEntry {
        mileage: end_mileage,
        fill_litres,
        fill_price,
        result_price: None,
    })) {
        let miles_to_here = entry.mileage - tank_price_at;
        let fuel_to_here = miles_to_here as f64 / mpl;

        println!("{}, {}", fuel_to_here, tank_price);
        used_cost += fuel_to_here * tank_price;

        let remaining_fuel = tank_amount - fuel_to_here;
        let remaining_fuel_cost = tank_price * remaining_fuel;

        let added_fuel = entry.fill_litres;
        let added_fuel_cost = entry.fill_price * added_fuel;

        let total_fuel = remaining_fuel + added_fuel;
        let total_fuel_cost = remaining_fuel_cost + added_fuel_cost;

        let total_fuel_price = total_fuel_cost / total_fuel;

        tank_price_at = entry.mileage;
        tank_price = total_fuel_price;
        tank_amount = total_fuel;
    }

    println!("uc {used_cost}, tl {total_litres}");
    let perlitre = used_cost / used_litres;
    let permile = used_cost / total_mileage as f64;
    let perkm = used_cost / total_mileage as f64 * 0.62137119;

    // let result_price = ((TANK_SIZE - used_litres) * start_price + used_litres * fill_price) / TANK_SIZE;

    MpgStats {
        total_mileage,
        used_litres,
        used_cost,

        mpg,
        mpg_us,
        lp100km,

        perlitre,
        permile,
        perkm,

        result_price: tank_price,
    }
}
