use poise::{serenity_prelude as serenity, PopArgument};
use std::env;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn selling(
    ctx: Context<'_>,
    #[description = "sale quantity"] sale_quantity: i32,
    #[description = "sale item"] sale_item: String,
    #[description = "buy quantity"] buy_quantity: i32,
    #[description = "buy item"] buy_item: String,
    #[description = "location north"] location_north: i32,
    #[description = "location east"] location_east: i32,
) -> Result<(), Error> {
    println!("sale_quantity: {}, sale_item: {}, buy_quantity: {}, buy_item: {}", sale_quantity, sale_item, buy_quantity, buy_item);
    let response = format!("Selling {}", sale_quantity);
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Starting up...");

    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![selling()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    println!("Awaiting messages...");
    client.unwrap().start().await.unwrap();
}

// T1 Flint Saw
// T2 Pyrelite Saw
// T2 Perfected Pyrelite Saw
// T3 Aurumite Saw
// T3 Perfected Aurumite Saw
// T4 Ferralith Saw
// T4 Perfected Ferralith Saw

// T1 Flint Pot
// T2 Pyrelite Pot
// T2 Perfected Pyrelite Pot
// T3 Aurumite Pot
// T3 Perfected Aurumite Pot
// T4 Ferralith Pot
// T4 Perfected Ferralith Pot

// T1 Flint Hoe
// T2 Pyrelite Hoe
// T2 Perfected Pyrelite Hoe
// T3 Aurumite Hoe
// T3 Perfected Aurumite Hoe
// T4 Ferralith Hoe
// T4 Perfected Ferralith Hoe

// T1 Flint Rod
// T2 Pyrelite Rod
// T2 Perfected Pyrelite Rod
// T3 Aurumite Rod
// T3 Perfected Aurumite Rod
// T4 Ferralith Rod
// T4 Perfected Ferralith Rod

// T1 Flint Machete
// T2 Pyrelite Machete
// T2 Perfected Pyrelite Machete
// T3 Aurumite Machete
// T3 Perfected Aurumite Machete
// T4 Ferralith Machete
// T4 Perfected Ferralith Machete

// T1 Flint Bow
// T2 Pyrelite Bow
// T2 Perfected Pyrelite Bow
// T3 Aurumite Bow
// T3 Perfected Aurumite Bow
// T4 Ferralith Bow
// T4 Perfected Ferralith Bow

// T1 Flint Knife
// T2 Pyrelite Knife
// T2 Perfected Pyrelite Knife
// T3 Aurumite Knife
// T3 Perfected Aurumite Knife
// T4 Ferralith Knife
// T4 Perfected Ferralith Knife

// T1 Flint Chisel
// T2 Pyrelite Chisel
// T2 Perfected Pyrelite Chisel
// T3 Aurumite Chisel
// T3 Perfected Aurumite Chisel
// T4 Ferralith Chisel
// T4 Perfected Ferralith Chisel

// T1 Flint Pickaxe
// T2 Pyrelite Pickaxe
// T2 Perfected Pyrelite Pickaxe
// T3 Aurumite Pickaxe
// T3 Perfected Aurumite Pickaxe
// T4 Ferralith Pickaxe
// T4 Perfected Ferralith Pickaxe

// T1 Flint Quill
// T2 Pyrelite Quill
// T2 Perfected Pyrelite Quill
// T3 Aurumite Quill
// T3 Perfected Aurumite Quill
// T4 Ferralith Quill
// T4 Perfected Ferralith Quill

// T1 Flint Hammer
// T2 Pyrelite Hammer
// T2 Perfected Pyrelite Hammer
// T3 Aurumite Hammer
// T3 Perfected Aurumite Hammer
// T4 Ferralith Hammer
// T4 Perfected Ferralith Hammer

// T1 Flint Scissors
// T2 Pyrelite Scissors
// T2 Perfected Pyrelite Scissors
// T3 Aurumite Scissors
// T3 Perfected Aurumite Scissors
// T4 Ferralith Scissors
// T4 Perfected Ferralith Scissors

// Schematic: Perfected Pyrelite Saw
// Schematic: Perfected Pyrelite Pot
// Schematic: Perfected Pyrelite Hoe
// Schematic: Perfected Pyrelite Rod
// Schematic: Perfected Pyrelite Machete
// Schematic: Perfected Pyrelite Bow
// Schematic: Perfected Pyrelite Knife
// Schematic: Perfected Pyrelite Chisel
// Schematic: Perfected Pyrelite Pickaxe
// Schematic: Perfected Pyrelite Quill
// Schematic: Perfected Pyrelite Hammer
// Schematic: Perfected Pyrelite Scissors

// T1 Badly Damaged Head Power Source
// T1 Badly Damaged Hand Power Source
// T1 Badly Damaged Torso Power Source
// T1 Badly Damaged Leg Power Source

// Clay Lump
// Flint Tool Bundle
// Knapped Flint
// Rough Plant Fiber
// Flint Axe
// Primitive Mallet
// Basic flower
// Wild fiber plant seeds
// Rough parchment
// Rough tree bark
// Rough twine
// Rough woodlog
// Basic pigment
// Basic fertilizer
// Oak seed
// Potted Oak Sapling
// Rough foresters pot
// Basic tannin
// Basic fish oil 
// Rough rope
// Rough fabric
// Rough charcoal
// Plain mushroom skewer
// Rough untreated plank
// Rough spool of thread
// Basic wispweave plant
// Rough plank. 
// Pyrelite nails 
// Rough pebbles
// Basic Berry 
// Plain cooked berries
// Plain trail mix
// Wild grain
// Moonlit crawdad
// Plain skewered baitfish
// Basic bait
// Rough shell
// Crushed rough shells
// Basic crop oil
// Basic embergrain plant
// Wild grain seeds
// Basic starbulb plant
// Basic starbulb seeds
// Basic wispweave seeds
// Basic embergrain seeds
// Wild vegetable seeds
// Basic mushroom
// Rough repaired ancient wires
// Rough repaired ancient spring
// Rough repaired ancient resistor
// Rough repaired hand power source
// Rough repaired ancient mechanism
// Rough repaired leg power source
// Rough repaired head power source
// Rough repaired ancient gear
// Rough glowing module
// Rough repaired torso power source
// Rough repaired ancient
// Breezy Fin
// Breezy Fin darter products
// Basic Chum
// Basic raw meat
// Rough cloth
// Rough leather straps
// Salt
// Plain mushroom stuffed bulbs
// Basic starbulb
// Pyrelite ore concentrate
// Molten Pyrelite
// Basic ink
// Beginners study journal
// Beginner's stone carvings
// Rough raw pelt 
// Rough cleaned pelt
// Crushed pyrelite ore
// Rough rock
// Basic mortar
// Plain mashed bulbs
// Plain vegetable stew
// Rough wispweave filament
// Rough cloth torso power source
// Rough cloth hand power source
// Rough tool handle
// Rough cloth head power source
// Rough cloth leg power source
// Pointed
// Tattered cap
// Pointed cap
// Coconut hat
// Shirt and suspenders
// Tropical shirt and skirt
// Ocean footwraps
// Ocean belt
// Ragged shorts
// Tattered shorts
// Deed: cart
// Deed: Skiff
// Professional's Mallet
// Deed: Expert's Skiff
// Deed: Raft
// Craftsman's Mallet
// Deed: Large cart
// Experts Mallet
// Deed: Masterful skiff
// Deed: Advanced sciff
// Brico's Mallet
// Plain Roasted meat
// Rough Stone head power source
// Rough Stone leg power source
// Rough Stone hand power source
// Rough Stone torso power source
// Rough dried pelt
// Breezy Finn Darter Filet
// Plain roasted fish
// Plain hot tea
// Plain chilling tea
// Rough leather
// Rough leather head power source
// Rough leather leg power source
// Rough leather hand power source
// Rough leather torso power source
// High quality rough plank
// Pyrelite ingot
// High quality rough cloth
// High quality rough cloth head power source
// High quality rough cloth leg power source
// High quality rough cloth hand power source
// High quality rough cloth torso power source
// Grand rough diamond
// Massive rough Ruby
// Fancy white hat
// Ancient armwraps
// Fancy shirt
// Oceancrest Marlin Scale
// Rough brick
// Rough unfired Forester's pot. 
// Beech seed
// Potted beech sapling
// Plain bread
// Blacksmith's key recipe
// Schematic: rough overloaded leather module 
// Birch seed
// Potted Birch sapling
// Deed: fisher's boat
// Deed: Improved skiff
// Brico's Old Mallet
// Plain ground meat and mashed bulbs
// Plain raw dumpling
// Basic Embergrain dough
// Plain meat sandwich
// Plain Fish and bulbs
// Plain roasted ocean fish