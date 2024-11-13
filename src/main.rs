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
    Regex::new(r"(?<numDice>\d+)?d?(?<numSides>\d+)?(?<modifier>[+-]\d+)?").unwrap()
});
static WHITESPACE_REGULAR_EX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+").unwrap()
});
static PLUS_MINUS_REGULAR_EX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?<minus>-\d+d\d+|-\d+)|(?<plus>\+?\d+d\d+|\+?\d+)").unwrap()
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
    added_dice: bool,
}

impl Dice{ // Jak jest faces == 0 to można by tak zapisywać modyfikatory! :D i w tedy nawet zwykłe +5 traktować jak rzut osobny :D !!!!
    fn new( faces: u8, added_dice: bool ) -> Self {
        Dice{
            faces,
            roll_result: Some(0),
            added_dice,
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

impl Roll{ // Tutaj mamy podzielić jeden rzut na kości i potem je przekalkulować i podać wynik ...
    fn new( original_roll: String ) -> Self { // For now it only deals with one roll of format 2d6+1
        let mut modifier: i8 = 0;
        let mut faces: u8 = 0;
        let mut numOfDices: usize = 0;
        let stripped_string = original_roll.replace(" ", "");


        let plus_minus = Self::parse_roll(&original_roll);
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
            match caps.name("numSides"){
                Some(x) => {
                    match x.as_str().parse::<u8>(){
                        Ok(n) => faces = n,
                        Err(e) => faces = 0,
                    }
                },
                None => faces = 0,
                
            }
            match caps.name("numDice"){
                Some(x) => match x.as_str().parse::<usize>(){
                    Ok(n) => numOfDices = n,
                    Err(e) => numOfDices = 0,
                },
                None => faces = 0,
            }
        }

        Roll {
            original_roll,
            dices: {
                Self::roles_processing(plus_minus)
            },
            modifier: modifier,
        }
    }

    fn parse_roll( full_role: &String) -> (Vec<String>,Vec<String>){// Dostajemy coś w stylu -1 + 2d6 -1d4 +2- 1+ 11d2

        let clean_role = WHITESPACE_REGULAR_EX.replace_all(&full_role, "");

        let mut plus = Vec::new();
        let mut minus = Vec::new();
        for caps in PLUS_MINUS_REGULAR_EX.captures_iter(clean_role.into_owned().as_str()){
            if let Some(val) = caps.name("plus") {
                plus.push(val.as_str().to_string());
            }
            else{
                if let Some(val) = caps.name("minus") {
                    minus.push(val.as_str().to_string());
                }
            }
        }
        
        (plus,minus)
    }

    fn roles_processing( plus_minus: (Vec<String>,Vec<String>) ) -> Vec<Dice>{
        let (plus, minus) = plus_minus;
        let mut res = Vec::new();
        plus.iter().for_each(|x| res.extend(Self::dice_from_string(x)) );
        minus.iter().for_each(|x| res.extend(Self::dice_from_string(x)) );

        res
    }

    fn dice_from_string( single_dice: &String) -> Vec<Dice> {

        let mut res = Vec::new();
        let mut stripped_string;

        let mut added_dice = true;
        let mut faces = 0;
        let mut roll_result= Some(0);
        let mut num_of_dices = 1;

        if single_dice.starts_with('-') {
            added_dice = false;
            stripped_string = single_dice.strip_prefix('-').unwrap();
        } else{
            stripped_string = single_dice.as_str().clone();
        }

        if let Some(caps) = DICE_ROLL_REGULAR_EX.captures(&stripped_string){
            match caps.name("modifier"){
                Some(x) => {
                    match x.as_str().parse::<u8>(){
                        Ok(n) => roll_result = Some(n),
                        Err(e) => {},
                    }
                },
                None => {},
            } 
            match caps.name("numSides"){
                Some(x) => {
                    match x.as_str().parse::<u8>(){
                        Ok(n) => faces = n,
                        Err(e) => {},
                    }
                },
                None => {},
                
            }
            match caps.name("numDice"){
                Some(x) => match x.as_str().parse::<usize>(){
                    Ok(n) => num_of_dices = n,
                    Err(e) => {},
                },
                None => {},
            }
        }

        for i in 0..num_of_dices {
            res.push( Dice::new(faces, added_dice) );
            if roll_result != Some(0) {
                res[i].roll_result = roll_result;
            }
        }

        res
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

    //fn parse_roll( &self ) {
//
    //}

    fn perform( &mut self ){
        self.rolls.iter_mut().for_each( |x| x.roll_all() );
    }

    fn to_string( &self ) -> String { // Klasa piszaca dla Roll + zbieranie wyników dla sumy :D i już
        format!( "Reult: {} of role: {}", <u8 as TryInto<i8>>::try_into(self.rolls.iter().flat_map(|x| x.dices.iter()).map(|x| x.roll_result.unwrap()).sum::<u8>()).unwrap() + self.rolls.iter().map(|x| x.modifier).sum::<i8>(), self.original_rolls )
    } //// TUTAJ TRZEBA POPRAWIC BO NIE MA UWZGLEDNIENIA MINUS PLUS NA PODSTAWIE BOOLA NA DICE!!!
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

