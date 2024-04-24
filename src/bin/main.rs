use clokwerk::AsyncScheduler;
use clokwerk::TimeUnits;
use csv::ReaderBuilder;
use futures::Stream;
use memchr::memmem;
use poise::serenity_prelude as serenity;
use prettytable::format;
use prettytable::row;
use prettytable::Table;
use rusqlite::{params, Connection, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use std::{env, fs::File};

struct Data {
    item_list: HashMap<String, bool>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
struct Listing {
    id: i32,
    offer_quantity: i32,
    offer_item: String,
    request_quantity: i32,
    request_item: String,
    location_north: i32,
    location_east: i32,
    user: String,
    offer_count: i32,
    description: String,
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

    let cargo_items =
        load_items_from_file("items_cargo_data_utf16.txt").expect("Could not load items");
    let item_items =
        load_items_from_file("items_item_data_utf16.txt").expect("Could not load items");
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
                nearby_listings(),
                info(),
                my_listings(),
                help(),
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
            description text DEFAULT '',
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
                    "DELETE FROM listings WHERE timestamp <= datetime('now', '-5 days')",
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

/// Help command
#[poise::command(slash_command, prefix_command)]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let help_message = "
    Use a forward slash '/' to use BRT commands. BRT will respond in DMs or server channels.

    1. /list - Creates a new listing to advertise to other players. Offer count is optional!
        (ex: /list sale_quantity: 1 sale_item: Rough Cloth (T1) buy_quantity: 100 buy_item: Hex Coin location_north: 1000 location_east: 1000)

    2. /unlist - Remove one of your own listings. Use /my_listings to get the IDs of your listings.
        (ex: /unlist listing_id: 10)

    3. /info - Get more information about a listing by ID.
        (ex: /info listing_id: 10)

    4. /my_listings - Display a list of your own listings
        (ex: /my_listings)

    5. /nearby_listings - Search nearby for any available listings.
        (ex: /nearby_listings location_north: 1000 location_east: 1000 distance: 100)

    6. /nearby_sellers - Search nearby for users interested in selling the specified item.
        (/nearby_sellers sale_item: Rough Cloth (T1) location_north: 1000 location_east: 1000 distance: 100)

    8. /nearby_buyers - Search nearby for users interested in buying the specified item.
        (ex: /nearby_buyers buy_item: Rough Cloth (T1) location_north: 1000 location_east: 1000 distance: 100)

    8. /help - Display this message :)
    ";
    ctx.say(help_message).await?;
    Ok(())
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
        ctx.say("Listing successfully unlisted").await?;
    } else {
        ctx.say("Listing not found").await?;
    }
    Ok(())
}

/// Post a listing!
#[poise::command(slash_command, prefix_command)]
async fn list(
    ctx: Context<'_>,
    #[description = "offer quantity"] offer_quantity: i32,
    #[description = "offer item"]
    #[autocomplete = "autocomplete_item_name"]
    offer_item: String,
    #[description = "request quantity"] request_quantity: i32,
    #[description = "request item"]
    #[autocomplete = "autocomplete_item_name"]
    request_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "offer count"] offer_count: Option<i32>,
    #[description = "description"] description: Option<String>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let username = ctx.author().name.clone();
    let description = description.unwrap_or("".to_string());
    if description.len() > 300 {
        ctx.say("Invalid listing: Description must be 300 characters or less").await?;
        return Ok(());
    }

    if offer_item == request_item {
        ctx.say("Invalid listing: Offered item cannot be the same as the requested item")
            .await?;
        return Ok(());
    }

    if request_quantity == 0 || offer_quantity == 0 {
        ctx.say("Invalid listing: Request quantity and offer quantity must be non-zero")
            .await?;
        return Ok(());
    }

    let db = Connection::open("db.db3")?;

    let listing_count: i32 = db.query_row(
        "SELECT COUNT(*) FROM listings WHERE username = ?",
        params![username],
        |row| row.get(0),
    )?;

    if !ctx.author().name.contains("cyypherus") && listing_count >= 15 {
        ctx.say(format!(
            "You have reached the maximum number of listings (15). You can remove some with /my_listings & /unlist"
        ))
        .await?;
        return Ok(());
    }

    let offer_count = offer_count.unwrap_or(1);

    if ctx.data().item_list.contains_key(&request_item)
        && ctx.data().item_list.contains_key(&offer_item)
    {
        let listing_info = format_listings(
            vec![Listing {
                id: 0,
                offer_quantity,
                offer_item: offer_item.clone(),
                request_quantity,
                request_item: request_item.clone(),
                location_north,
                location_east,
                user: username.clone(),
                offer_count: offer_count,
                description: description.clone(),
            }],
            0,
        );
        db.execute(
            "INSERT INTO listings (sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, timestamp, offer_count, description)
            VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?)",
            params![
            offer_quantity,
            offer_item,
            request_quantity,
            request_item,
            location_north,
            location_east,
            username,
            offer_count,
            description,
            ],
        )?;
        println!("{}", listing_info);
        ctx.say(format!(
            "Listing successful! Your listing will expire in 5 days\n{}",
            listing_info,
        ))
        .await?;
        Ok(())
    } else {
        let error_message = format!("Items {} and/or {} not found.", request_item, offer_item,);
        ctx.say(error_message).await?;
        return Ok(());
    }
}

/// Check your own listings
#[poise::command(slash_command)]
async fn my_listings(
    ctx: Context<'_>,
    #[description = "page"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let username = ctx.author().name.clone();
    let db = Connection::open("db.db3")?;
    let listings = query_listings_by_username(&db, &username)?;
    if listings.is_empty() {
        ctx.say("You have no listings.").await?;
    } else {
        let listings_info = format_listings(listings, page.unwrap_or(1));
        ctx.say(listings_info).await?;
    }
    Ok(())
}

/// Query listings by username
fn query_listings_by_username(db: &Connection, username: &str) -> Result<Vec<Listing>, Error> {
    let mut stmt = db.prepare(
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, offer_count, description
        FROM listings
        WHERE username = ?",
    )?;
    let queries = stmt.query_map(params![username], |row| {
        Ok(Listing {
            id: row.get(0)?,
            offer_quantity: row.get(1)?,
            offer_item: row.get(2)?,
            request_quantity: row.get(3)?,
            request_item: row.get(4)?,
            location_north: row.get(5)?,
            location_east: row.get(6)?,
            user: username.to_string(),
            offer_count: row.get(7)?,
            description: row.get(8)?,
        })
    })?;
    let mut listings = Vec::<Listing>::new();
    for q in queries {
        listings.push(q?);
    }
    Ok(listings)
}
/// Search nearby listings
#[poise::command(slash_command, prefix_command)]
async fn nearby_listings(
    ctx: Context<'_>,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
    #[description = "distance"] distance: i32,
    #[description = "page"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    let rows = get_all_listings_within_distance(&db, location_north, location_east, distance)?;
    if rows.is_empty() {
        ctx.say(format!(
            "No listings found within N ({} - {}) E ({} - {})",
            location_north - distance,
            location_north + distance,
            location_east - distance,
            location_east + distance,
        ))
        .await?;
    } else {
        let listings_info = format_listings(rows, page.unwrap_or(1));
        ctx.say(format!("Nearby listings:\n{}", listings_info,))
            .await?;
    }
    Ok(())
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
    #[description = "page"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&sale_item) {
        let error_message = format!("Item {} not found", sale_item);
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
            "No sellers of {} found within N ({} - {}) E ({} - {})",
            sale_item,
            location_north - distance,
            location_north + distance,
            location_east - distance,
            location_east + distance,
        ))
        .await?;
    } else {
        let sellers_info = format_listings(rows, page.unwrap_or(1));
        ctx.say(format!("Nearby sellers:\n{}", sellers_info,))
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
    #[description = "page"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // Search for listings within distance of location
    let db = Connection::open("db.db3")?;

    if !ctx.data().item_list.contains_key(&buy_item) {
        let error_message = format!("Item {} not found", buy_item);
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
            "No buyers of {} found within N ({} - {}) E ({} - {}).",
            buy_item,
            location_north - distance,
            location_north + distance,
            location_east - distance,
            location_east + distance,
        ))
        .await?;
    } else {
        let buyers_info = format_listings(rows, page.unwrap_or(1));
        ctx.say(format!("Nearby buyers:\n{}", buyers_info,)).await?;
    }
    Ok(())
}

/// Get info on a listing by ID
#[poise::command(slash_command, prefix_command)]
async fn info(
    ctx: Context<'_>,
    #[description = "listing ID"] listing_id: i32,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    
    let db = Connection::open("db.db3")?;
    let listing = get_listing_by_id(&db, listing_id)?;
    
    if let Some(listing) = listing {
        let mut info = format!("```Description: {}\n", listing.description);
        info.push_str(&format!(
            "Offer: {} {}\nRequest: {} {}\nLocation: N:{} E:{}\nStock: {}\nUser: {}\n",
            listing.offer_quantity,
            listing.offer_item,
            listing.request_quantity,
            listing.request_item,
            listing.location_north,
            listing.location_east,
            listing.offer_count,
            listing.user,
        ));
        info.push_str("```");
        ctx.say(info).await?;
    } else {
        ctx.say("Listing not found").await?;
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
fn get_all_listings_within_distance(
    db: &Connection,
    location_north: i32,
    location_east: i32,
    distance: i32,
) -> Result<Vec<Listing>, Error> {
    let mut stmt = db.prepare(
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, offer_count, description
        FROM listings
        WHERE
        ABS(location_north - (?1)) <= (?2) AND ABS(location_east - (?3)) <= (?4)",
    )?;
    let queries = stmt.query_map(
        params![location_north, distance, location_east, distance,],
        |row| {
            Ok(Listing {
                id: row.get(0)?,
                offer_quantity: row.get(1)?,
                offer_item: row.get(2)?,
                request_quantity: row.get(3)?,
                request_item: row.get(4)?,
                location_north: row.get(5)?,
                location_east: row.get(6)?,
                user: row.get(7)?,
                offer_count: row.get(8)?,
                description: row.get(9)?,
            })
        },
    )?;
    let mut local_rows = Vec::<Listing>::new();
    for q in queries {
        local_rows.push(q?);
    }
    Ok(local_rows)
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
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, offer_count, description
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
                offer_quantity: row.get(1)?,
                offer_item: row.get(2)?,
                request_quantity: row.get(3)?,
                request_item: row.get(4)?,
                location_north: row.get(5)?,
                location_east: row.get(6)?,
                user: row.get(7)?,
                offer_count: row.get(8)?,
                description: row.get(9)?,
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
        .filter(|item| {
            finder
                .find_iter(item.to_lowercase().as_bytes())
                .next()
                .is_some()
        })
        .collect::<Vec<String>>();
    item_list.truncate(15);
    futures::stream::iter(item_list)
}

/// Get a listing by ID
fn get_listing_by_id(db: &Connection, listing_id: i32) -> Result<Option<Listing>, Error> {
    let mut stmt = db.prepare(
        "SELECT id, sale_quantity, sale_item, buy_quantity, buy_item, location_north, location_east, username, offer_count, description
        FROM listings
        WHERE id = ?1",
    )?;
    let query = stmt.query_row(params![listing_id], |row| {
        Ok(Listing {
            id: row.get(0)?,
            offer_quantity: row.get(1)?,
            offer_item: row.get(2)?,
            request_quantity: row.get(3)?,
            request_item: row.get(4)?,
            location_north: row.get(5)?,
            location_east: row.get(6)?,
            user: row.get(7)?,
            offer_count: row.get(8)?,
            description: row.get(9)?,
        })
    });
    
    match query {
        Ok(listing) => Ok(Some(listing)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(Box::new(err)),
    }
}

/// Format a vector of listings into a string for display
fn format_listings(listings: Vec<Listing>, page: i32) -> String {
    let user_page = page;
    let page = page - 1;
    let mut i = 0;
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);
    table.add_row(row![
        "Offer",
        "Request",
        "Location",
        "ID"
    ]);
    let mut pages = Vec::<String>::new();
    for listing in listings {
        let row = row![
            format!("{} {}", listing.offer_quantity, listing.offer_item),
            format!("{} {}", listing.request_quantity, listing.request_item),
            format!("N:{} E:{}", listing.location_north, listing.location_east),
            listing.id
        ];
        table.add_row(row.clone());
        i += 1;
        if table.to_string().len() > 2000 {
            let removed_1 = table.get_row(i).unwrap().clone();
            let removed_2 = table.get_row(i - 1).unwrap().clone();
            table.remove_row(i);
            table.remove_row(i - 1);
            table.add_row(row![format!("Show more... add page:{} to your command", user_page + 1)]);
            pages.push(format!("```\n{}\n```", table.to_string()));
            table = Table::new();
            table.set_format(*format::consts::FORMAT_CLEAN);
            table.add_row(row![
                "Offer",
                "Request",
                "Location",
                "ID"
            ]);
            table.add_row(removed_2);
            table.add_row(removed_1);
        }
    }
    pages.push(format!("```\n{}\n```", table.to_string()));
    pages[(page as usize).min(pages.len() - 1)].clone()
}