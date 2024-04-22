use clokwerk::AsyncScheduler;
use clokwerk::TimeUnits;
use poise::serenity_prelude as serenity;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::time::Duration;
use std::{env, fs::File, io::Read};
use futures::{Stream, StreamExt};


struct Data {
    item_list: HashMap<String, bool>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn autocomplete_item_name<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let item_list = ctx.data().item_list.keys().cloned().collect::<Vec<String>>();
    println!("Autocomplete item name function called");
    futures::stream::iter(item_list)
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.to_string())
}

/// Post a listing!
#[poise::command(slash_command, prefix_command)]
async fn selling(
    ctx: Context<'_>,
    #[description = "sale quantity"] 
    sale_quantity: i32,
    #[description = "sale item"] 
    #[autocomplete = "autocomplete_item_name"]
    sale_item: String,
    #[description = "buy quantity"] 
    buy_quantity: i32,
    #[description = "buy item"] 
    #[autocomplete = "autocomplete_item_name"]
    buy_item: String,
    #[description = "location north"] 
    location_north: i32,
    #[description = "location east"] 
    location_east: i32,
) -> Result<(), Error> {
    let db = Connection::open("db.db3")?;
    if ctx.data().item_list.contains_key(&buy_item) && ctx.data().item_list.contains_key(&sale_item)
    {
        let listing_info = format!(
            "\nSale: {} x {}\nBuy: {} x {}\nLocation: ({}, {})\n",
            sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east
        );
        println!("{}", listing_info);
        db.execute(
            "INSERT INTO sell_listings (sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
            params![
                sale_quantity,
                sale_item,
                buy_quantity,
                buy_item,
                location_north,
                location_east
            ],
        )?;
        ctx.say("Listing successful! Your listing will expire in 24 hours.")
            .await?;
        Ok(())
    } else {
        let error_message = format!("Items {} and/or {} not found", buy_item, sale_item);
        ctx.say(error_message).await?;
        return Ok(());
    }
}

struct Listing {
    sale_quantity: i32,
    sale_item: String,
    buy_quantity: i32,
    buy_item: String,
    location_north: i32,
    location_east: i32,
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
        "SELECT sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east
        FROM sell_listings
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
                sale_quantity: row.get(0)?,
                sale_item: row.get(1)?,
                buy_quantity: row.get(2)?,
                buy_item: row.get(3)?,
                location_north: row.get(4)?,
                location_east: row.get(5)?,
            })
        },
    )?;
    let mut local_rows = Vec::<Listing>::new();
    for q in queries {
        local_rows.push(q?);
    }
    Ok(local_rows)
}

enum ItemQuery {
    SellingItem,
    BuyingItem,
}

#[poise::command(slash_command, prefix_command)]
async fn nearby_sellers(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item_name"]
    #[description = "sale item"] sale_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "distance"] distance: i32,
) -> Result<(), Error> {
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&sale_item) {
        let error_message = format!("Item {} not found", sale_item);
        ctx.say(error_message).await?;
        return Ok(());
    }

    let rows = get_listings_within_distance(&db, &sale_item, location_north, location_east, distance, ItemQuery::SellingItem)?;
    if rows.is_empty() {
        ctx.say(
            format!(
                "No sellers of {} found within N ({} - {}) E ({} - {}).", 
                sale_item, 
                location_north - distance, 
                location_north + distance,
                location_east - distance, 
                location_east + distance,
            )
        ).await?;
    } else {
        let sellers_info = format_listings(rows);
        ctx.say(format!("Nearby sellers:\n{}", sellers_info)).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn nearby_buyers(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_item_name"]
    #[description = "buy item"] buy_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "distance"] distance: i32,
) -> Result<(), Error> {
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&buy_item) {
        let error_message = format!("Item {} not found", buy_item);
        ctx.say(error_message).await?;
        return Ok(());
    }

    let rows = get_listings_within_distance(&db, &buy_item, location_north, location_east, distance, ItemQuery::BuyingItem)?;
    if rows.is_empty() {
        ctx.say(
            format!(
                "No buyers of {} found within N ({} - {}) E ({} - {}).", 
                buy_item, 
                location_north - distance, 
                location_north + distance,
                location_east - distance, 
                location_east + distance,
            )
        ).await?;
    } else {
        let buyers_info = format_listings(rows);
        ctx.say(format!("Nearby buyers:\n{}", buyers_info)).await?;
    }
    Ok(())
}

/// Format a vector of listings into a string for display
fn format_listings(listings: Vec<Listing>) -> String {
    let mut formatted_listings = String::new();
    for listing in listings {
        let listing_info = format!(
            "Sale: {} x {}\nBuy: {} x {}\nLocation: ({}, {})\n\n",
            listing.sale_quantity,
            listing.sale_item,
            listing.buy_quantity,
            listing.buy_item,
            listing.location_north,
            listing.location_east
        );
        formatted_listings.push_str(&listing_info);
    }
    formatted_listings
}

#[tokio::main]
async fn main() {
    println!("Starting up...");
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    println!("Loading items...");
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let file_path = current_dir.join("modified_items.txt");
    let mut file = File::open(file_path).expect("Failed to open file");
    let mut item_strings = String::new();
    file.read_to_string(&mut item_strings)
        .expect("Failed to read file");
    let mut item_map: HashMap<String, bool> = HashMap::new();
    for item in item_strings.lines() {
        item_map.insert(item.to_string(), true);
    }
    let data = Data {
        item_list: item_map,
    };

    let intents = serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![selling(), nearby_buyers(), nearby_sellers()],
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
        "CREATE TABLE IF NOT EXISTS sell_listings (
            sale_quantity int,
            sale_item text,
            buy_quantity int,
            buy_item text,
            location_north int,
            location_east int,
            timestamp timestamp DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )
    .expect("Table create failed");

    let mut scheduler = AsyncScheduler::new();
    scheduler.every(1.minutes()).run(move || {
        Box::pin(async move {
            println!("Cleaning up old listings");
            let db = Connection::open("db.db3").expect("Db failed");
            let deleted_listings = db
                .execute(
                    "DELETE FROM sell_listings WHERE timestamp <= datetime('now', '-24 hours')",
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
