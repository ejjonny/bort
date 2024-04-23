use memchr::memmem;
use ::serenity::all::Mentionable;
use clokwerk::AsyncScheduler;
use clokwerk::TimeUnits;
use csv::ReaderBuilder;
use futures::Stream;
use poise::serenity_prelude as serenity;
use rusqlite::{params, Connection, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use std::{env, fs::File};

// Interacting with BRT:
// 1. /list - Post a listing!
//    - Description: Creates a new listing with the specified details. The sale_quantity and buy_quantity should be non-zero. The offer_count is optional and defaults to 1.
//    - Example: /list sale_quantity: 3 sale_item: Rough Cloth (T1) buy_quantity: 100 buy_item: Hex Coin location_north: 1000 location_east: 1000 offer_count: 20
//
// 2. /unlist - Unlist a listing
//    - Usage: /unlist <listing_id>
//    - Description: Removes the listing with the specified ID. Only the owner of the listing can unlist it.
//
// 3. /my_listings - Check your own listings
//    - Usage: /my_listings
//    - Description: Displays a list of your own listings.
//
// 4. /nearby_sellers - Search nearby sellers
//    - Usage: /nearby_sellers <sale_item> <location_north> <location_east> <distance>
//    - Description: Searches for sellers of the specified item within the specified distance from the given location.
//
// 5. /nearby_buyers - Search nearby buyers
//    - Usage: /nearby_buyers <buy_item> <location_north> <location_east> <distance>
//    - Description: Searches for buyers of the specified item within the specified distance from the given location.

struct Data {
    item_list: HashMap<String, bool>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
struct Listing {
    id: i32,
    sale_quantity: i32,
    sale_item: String,
    buy_quantity: i32,
    buy_item: String,
    location_north: i32,
    location_east: i32,
    user: String,
    offer_count: i32,
}

enum ItemQuery {
    SellingItem,
    BuyingItem,
}

#[derive(Debug, Deserialize)]
struct Item {
    name: String,
    tier: i32,
}

#[tokio::main]
async fn main() {
    println!("Starting up...");
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    println!("Loading items...");

    let cargo_items = load_items_from_file("items_cargo_data_utf16.txt").expect("Could not load items");
    let item_items = load_items_from_file("items_item_data_utf16.txt").expect("Could not load items");
    let mut item_map: HashMap<String, bool> = HashMap::new();
    for item in cargo_items.iter().chain(item_items.iter()) {
        let name_with_tier = if item.tier != -1 {
            format!("{} (T{})", item.name, item.tier)
        } else {
            item.name.clone()
        };
        item_map.insert(name_with_tier, true);
    }

    let data = Data {
        item_list: item_map,
    };

    let intents = serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                list(),
                unlist(),
                nearby_buyers(),
                nearby_sellers(),
                my_listings(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    let db = Connection::open("db.db3").expect("Db failed");
    db.execute(
        "CREATE TABLE IF NOT EXISTS listings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sale_quantity int,
            sale_item text,
            buy_quantity int,
            buy_item text,
            location_north int,
            location_east int,
            username text,
            offer_count int,
            timestamp timestamp DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )
    .expect("Table create failed");

    let mut scheduler = AsyncScheduler::new();
    scheduler.every(10.minutes()).run(move || {
        Box::pin(async move {
            println!("Cleaning up old listings");
            let db = Connection::open("db.db3").expect("Db failed");
            let deleted_listings = db
                .execute(
                    "DELETE FROM listings WHERE timestamp <= datetime('now', '-24 hours')",
                    (),
                )
                .expect("Failed to delete old listings");
            println!("Deleted {} listings", deleted_listings);
        })
    });

    tokio::spawn(async move {
        loop {
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    println!("Awaiting messages...");
    client.unwrap().start().await.unwrap();
}

/// Unlist a listing
#[poise::command(slash_command, prefix_command)]
async fn unlist(
    ctx: Context<'_>,
    #[description = "listing ID"] listing_id: i32,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let username = ctx.author().name.clone();
    let db = Connection::open("db.db3")?;
    let result = db.execute(
        "DELETE FROM listings WHERE id = ? AND username = ?",
        params![listing_id, username],
    )?;
    if result > 0 {
        ctx.say(format!(
            "Listing successfully unlisted\n{}",
            ctx.author().mention()
        ))
        .await?;
    } else {
        ctx.say(format!("Listing not found\n{}", ctx.author().mention()))
            .await?;
    }
    Ok(())
}

/// Post a listing!
#[poise::command(slash_command, prefix_command)]
async fn list(
    ctx: Context<'_>,
    #[description = "sale quantity"] sale_quantity: i32,
    #[description = "sale item"]
    #[autocomplete = "autocomplete_item_name"]
    sale_item: String,
    #[description = "buy quantity"] buy_quantity: i32,
    #[description = "buy item"]
    #[autocomplete = "autocomplete_item_name"]
    buy_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "offer count"] offer_count: Option<i32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let username = ctx.author().name.clone();

    if sale_item == buy_item {
        ctx.say(format!(
            "Invalid listing: Sale item cannot be the same as the buy item\n{}",
            ctx.author().mention()
        ))
        .await?;
        return Ok(());
    }

    if buy_quantity == 0 || sale_quantity == 0 {
        ctx.say(format!(
            "Invalid listing: Buy quantity and sale quantity must be non-zero\n{}",
            ctx.author().mention()
        ))
        .await?;
        return Ok(());
    }

    let db = Connection::open("db.db3")?;

    let listing_count: i32 = db.query_row(
        "SELECT COUNT(*) FROM listings WHERE username = ?",
        params![username],
        |row| row.get(0),
    )?;

    if listing_count >= 15 {
        ctx.say(format!(
            "You have reached the maximum number of listings (15). You can remove some with /my_listings & /unlist\n{}",
            ctx.author().mention()
        ))
        .await?;
        return Ok(());
    }

    let offer_count = offer_count.unwrap_or(1);

    if ctx.data().item_list.contains_key(&buy_item) && ctx.data().item_list.contains_key(&sale_item)
    {
        let listing_info = format_listings(
            vec![Listing {
                id: 0,
                sale_quantity,
                sale_item: sale_item.clone(),
                buy_quantity,
                buy_item: buy_item.clone(),
                location_north,
                location_east,
                user: username.clone(),
                offer_count: offer_count,
            }],
            true,
            false,
        );
        db.execute(
            "INSERT INTO listings (sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, timestamp, offer_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?)",
            params![
                sale_quantity,
                sale_item,
                buy_quantity,
                buy_item,
                location_north,
                location_east,
                username,
                offer_count,
            ],
            )?;
        println!("{}", listing_info);
        ctx.say(format!(
            "Listing successful! Your listing will expire in 24 hours\n{}\n{}",
            listing_info,
            ctx.author().mention()
        ))
        .await?;
        Ok(())
    } else {
        let error_message = format!(
            "Items {} and/or {} not found.\n{}",
            buy_item,
            sale_item,
            ctx.author().mention()
        );
        ctx.say(format!("{}, {}", error_message, ctx.author().mention()))
            .await?;
        return Ok(());
    }
}

/// Check your own listings
#[poise::command(slash_command)]
async fn my_listings(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let username = ctx.author().name.clone();
    let db = Connection::open("db.db3")?;
    let listings = query_listings_by_username(&db, &username)?;
    if listings.is_empty() {
        ctx.say(format!("You have no listings.\n{}", ctx.author().mention()))
            .await?;
    } else {
        let listings_info = format_listings(listings, false, true);
        ctx.say(format!(
            "Your listings:\n{}\n{}",
            listings_info,
            ctx.author().mention()
        ))
        .await?;
    }
    Ok(())
}

/// Query listings by username
fn query_listings_by_username(db: &Connection, username: &str) -> Result<Vec<Listing>, Error> {
    let mut stmt = db.prepare(
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, offer_count
        FROM listings
        WHERE username = ?",
    )?;
    let queries = stmt.query_map(params![username], |row| {
        Ok(Listing {
            id: row.get(0)?,
            sale_quantity: row.get(1)?,
            sale_item: row.get(2)?,
            buy_quantity: row.get(3)?,
            buy_item: row.get(4)?,
            location_north: row.get(5)?,
            location_east: row.get(6)?,
            user: username.to_string(),
            offer_count: row.get(7)?,
        })
    })?;
    let mut listings = Vec::<Listing>::new();
    for q in queries {
        listings.push(q?);
    }
    Ok(listings)
}

/// Search nearby sellers
#[poise::command(slash_command, prefix_command)]
async fn nearby_sellers(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item_name"]
    #[description = "sale item"]
    sale_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "distance"] distance: i32,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&sale_item) {
        let error_message = format!("Item {} not found\n{}", sale_item, ctx.author().mention());
        ctx.say(error_message).await?;
        return Ok(());
    }

    let rows = get_listings_within_distance(
        &db,
        &sale_item,
        location_north,
        location_east,
        distance,
        ItemQuery::SellingItem,
    )?;
    if rows.is_empty() {
        ctx.say(format!(
            "No sellers of {} found within N ({} - {}) E ({} - {}).\n{}",
            sale_item,
            location_north - distance,
            location_north + distance,
            location_east - distance,
            location_east + distance,
            ctx.author().mention(),
        ))
        .await?;
    } else {
        let sellers_info = format_listings(rows, true, false);
        ctx.say(format!(
            "Nearby sellers:\n{}\n{}",
            sellers_info,
            ctx.author().mention()
        ))
        .await?;
    }
    Ok(())
}

/// Search nearby buyers
#[poise::command(slash_command, prefix_command)]
async fn nearby_buyers(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item_name"]
    #[description = "buy item"]
    buy_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "distance"] distance: i32,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&buy_item) {
        let error_message = format!("Item {} not found {}", buy_item, ctx.author().mention());
        ctx.say(error_message).await?;
        return Ok(());
    }

    let rows = get_listings_within_distance(
        &db,
        &buy_item,
        location_north,
        location_east,
        distance,
        ItemQuery::BuyingItem,
    )?;
    if rows.is_empty() {
        ctx.say(format!(
            "No buyers of {} found within N ({} - {}) E ({} - {}).\n{}",
            buy_item,
            location_north - distance,
            location_north + distance,
            location_east - distance,
            location_east + distance,
            ctx.author().mention(),
        ))
        .await?;
    } else {
        let buyers_info = format_listings(rows, true, false);
        ctx.say(format!(
            "Nearby buyers:\n{}\n{}",
            buyers_info,
            ctx.author().mention()
        ))
        .await?;
    }
    Ok(())
}

fn load_items_from_file(file_name: &str) -> Result<Vec<Item>, Error> {
    let mut items = Vec::<Item>::new();
    let mut cargo_file = File::open(file_name)?;
    let mut cargo_data_string = String::new();
    cargo_file.read_to_string(&mut cargo_data_string)?;
    let mut rdr = ReaderBuilder::new()
        .delimiter(b'|')
        .quoting(false)
        .has_headers(true)
        .from_reader(cargo_data_string.as_bytes());
    rdr.records().for_each(|result| {
        let record = result.expect("Failed to parse record");
        let item = Item {
            name: record[0].trim().to_string(),
            tier: record[1].trim().parse().expect("Failed to parse tier"),
        };
        items.push(item);
    });
    Ok(items)
}

/// Get listings within a certain distance of a location
fn get_listings_within_distance(
    db: &Connection,
    item: &str,
    location_north: i32,
    location_east: i32,
    distance: i32,
    listing_type: ItemQuery,
) -> Result<Vec<Listing>, Error> {
    let (search_buy_item, search_sell_item) = match listing_type {
        ItemQuery::BuyingItem => (true, false),
        ItemQuery::SellingItem => (false, true),
    };
    let mut stmt = db.prepare(
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, offer_count
        FROM listings
        WHERE 
        ((buy_item = ?5 AND ?6) OR (sale_item = ?5 AND ?7))
        AND ABS(location_north - (?1)) <= (?2) AND ABS(location_east - (?3)) <= (?4)",
    )?;
    let queries = stmt.query_map(
        params![
            location_north,
            distance,
            location_east,
            distance,
            item,
            search_buy_item,
            search_sell_item,
        ],
        |row| {
            Ok(Listing {
                id: row.get(0)?,
                sale_quantity: row.get(1)?,
                sale_item: row.get(2)?,
                buy_quantity: row.get(3)?,
                buy_item: row.get(4)?,
                location_north: row.get(5)?,
                location_east: row.get(6)?,
                user: row.get(7)?,
                offer_count: row.get(8)?,
            })
        },
    )?;
    let mut local_rows = Vec::<Listing>::new();
    for q in queries {
        local_rows.push(q?);
    }
    Ok(local_rows)
}

async fn autocomplete_item_name<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let lowercased = partial.to_lowercase();
    let finder = memmem::Finder::new(lowercased.as_str());
    let mut item_list = ctx
        .data()
        .item_list
        .keys()
        .cloned()
        .filter(|item| { finder.find_iter(item.to_lowercase().as_bytes()).next().is_some() })
        .collect::<Vec<String>>();
    item_list.truncate(15);
    futures::stream::iter(item_list)
}

/// Format a vector of listings into a string for display
fn format_listings(listings: Vec<Listing>, include_username: bool, include_id: bool) -> String {
    let mut formatted_listings = String::new();
    let max_listings = 40;
    let mut count = 0;
    for listing in listings {
        if count >= max_listings {
            formatted_listings.push_str("That's a lot of listings, I won't show them all");
            break;
        }
        let listing_info = format!(
            "\n{}Selling: ({} {} for: {} {}) x{}\nLocation: ({}, {})\n{}\n",
            if include_id {
                format!("ID: {}\n", listing.id.to_string())
            } else {
                String::new()
            },
            listing.sale_quantity,
            listing.sale_item,
            listing.buy_quantity,
            listing.buy_item,
            listing.offer_count,
            listing.location_north,
            listing.location_east,
            if include_username {
                format!("User: {}", listing.user)
            } else {
                String::new()
            }
        );
        formatted_listings.push_str(&listing_info);
        count += 1;
    }
    formatted_listings
}
