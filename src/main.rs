use dotenv::dotenv;
use serenity::all::EventHandler;
use std::env;
use std::sync::LazyLock;
use std::convert::TryInto;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    Client,
    model::gateway::GatewayIntents,
};
use rand::Rng;
use regex::Regex;


static DICE_ROLL_REGULAR_EX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?<numDice>\d+)d(?<numSides>\d+)(?<modifier>[+-]\d+)?").unwrap()
});


async fn send_to_discord( ctx: Context, msg: Message, answer: String ){
    if let Err(why) = msg.channel_id.say(&ctx.http, answer).await {
        println!("Error sending message: {:?}", why);
    }
}

#[derive(Clone)]
struct Dice{
    faces: u8,
    roll_result: Option<u8>,
}

impl Dice{ // Jak jest faces == 0 to można by tak zapisywać modyfikatory! :D i w tedy nawet zwykłe +5 traktować jak rzut osobny :D !!!!
    fn new( faces: u8 ) -> Self {
        Dice{
            faces,
            roll_result: Some(0),
        }
    }

    fn roll( &mut self ){
        if self.faces == 0 {
            return;
        }
        match roll_die(self.faces){
            x => self.roll_result = Some(x),
            _ => {},
        };
    }
}

fn roll_die(sides: u8) -> u8 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=sides)
}

#[derive(Clone)]
struct Roll{
    original_roll: String,
    dices: Vec<Dice>,
    modifier: i8,
}

impl Roll{
    fn new( original_roll: String ) -> Self { // For now it only deals with one roll of format 2d6+1
        let mut modifier: i8 = 0;
        let mut faces: u8 = 0;
        let mut numOfDices: usize = 0;
        let stripped_string = original_roll.replace(" ", "");

        if let Some(caps) = DICE_ROLL_REGULAR_EX.captures(&stripped_string){
            match caps.name("modifier"){
                Some(x) => {
                    match x.as_str().parse::<i8>(){
                        Ok(n) => modifier = n,
                        Err(e) => modifier = 0,
                    }
                },
                None => {},
            } 
            match caps["numSides"].parse::<u8>(){
                Ok(n) => faces = n,
                Err(e) => faces = 0,
            }
            match caps["numDice"].parse::<usize>(){
                Ok(n) => numOfDices = n,
                Err(e) => numOfDices = 0,
            }
        }

        Roll {
            original_roll,
            dices: {
                vec![Dice::new(faces); numOfDices]
            },
            modifier: modifier,
        }
    }

    fn roll_all(&mut self) {
        self.dices.iter_mut().for_each(|x| x.roll() );
    }
}

struct DiceRoll{
    original_rolls: String,
    rolls: Vec<Roll>,
}

impl DiceRoll{
    fn new( original_rolls: String ) -> Self {
        let original_rolls_clone = original_rolls.clone();
        Self {
            original_rolls,
            rolls: Vec::from([Roll::new( original_rolls_clone )])
        }
    }

    fn parse_roll( &self ) {

    }

    fn perform( &mut self ){
        self.rolls.iter_mut().for_each( |x| x.roll_all() );
    }

    fn to_string( &self ) -> String { // Klasa piszaca dla Roll + zbieranie wyników dla sumy :D i już
        format!( "Reult: {} of role: {}", <u8 as TryInto<i8>>::try_into(self.rolls.iter().flat_map(|x| x.dices.iter()).map(|x| x.roll_result.unwrap()).sum::<u8>()).unwrap() + self.rolls.iter().map(|x| x.modifier).sum::<i8>(), self.original_rolls )
    }
}

#[test]
fn dice_roller_test(){
    // 1d6
    assert_eq!("a","a");
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id == ctx.cache.current_user().id {
            return; // Exit early if the bot wrote this message
        }

        match msg.content.as_str() {
            x if x.starts_with("!hello") => {
                send_to_discord( ctx, msg, String::from("Hello! How can I help?") ).await;
            },
            x if x.starts_with("!roll") => {
                if x.len() < 7 {

                    return;
                }
                let mut roll_to_process = DiceRoll::new( x.to_string()[6..].to_string() );
                roll_to_process.perform();

                send_to_discord( ctx, msg, String::from( format!("{}", roll_to_process.to_string() ) ) ).await;
            },
            x if x.starts_with("!help") => {
                let help_to_process = x.to_string()[7..].to_string();

                send_to_discord( ctx, msg, String::from( format!("Help commend used with text: \"{}\"", help_to_process ) ) ).await;
            },
            _ => {
                println!("Error message of unknown kind: {:?}", msg.content);
                if let Err(why) = msg.channel_id.say(&ctx.http, format!("Sorry but commend \"{}\" is not knowen for me!", msg.content)).await {
                    println!("Error sending message: {:?}", why);
                }
            }
        };
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}
#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    //
    //GUILD_MESSAGES      0b00000001
    //MESSAGE_CONTENT     0b00000010

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

