//! Traveller NPC generation logic (Mongoose 2e–style career path, simplified).
//! Pure std so it can be unit-tested without the GUI; serde derives are
//! enabled when the `gui` feature (default) is on.

use std::collections::BTreeMap;

// ---------------------------------------------------------------- RNG ----
/// Small deterministic RNG (SplitMix64) so a seed always reproduces an NPC.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in 0..n
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn d6(&mut self) -> i32 {
        self.below(6) as i32 + 1
    }
    pub fn roll(&mut self, dice: u32) -> i32 {
        (0..dice).map(|_| self.d6()).sum()
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// A fresh seed from the clock (used when the seed isn't locked).
pub fn time_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    Rng::new(n).next_u64() % 1_000_000_000
}

// -------------------------------------------------------------- Stats ----
pub const STAT_NAMES: [&str; 6] = ["STR", "DEX", "END", "INT", "EDU", "SOC"];
pub const STR: usize = 0;
pub const DEX: usize = 1;
pub const END: usize = 2;
pub const INT: usize = 3;
pub const EDU: usize = 4;
pub const SOC: usize = 5;

pub fn stat_dm(v: i32) -> i32 {
    match v {
        i32::MIN..=0 => -3,
        1..=2 => -2,
        3..=5 => -1,
        6..=8 => 0,
        9..=11 => 1,
        12..=14 => 2,
        _ => 3,
    }
}

/// Traveller extended hex (skips I and O).
pub fn ehex(v: i32) -> char {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    DIGITS[v.clamp(0, DIGITS.len() as i32 - 1) as usize] as char
}

pub fn format_dm(dm: i32) -> String {
    if dm > 0 { format!("+{dm}") } else { format!("{dm}") }
}

pub fn format_credits(cr: i64) -> String {
    let s = cr.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("Cr{}{}", if cr < 0 { "-" } else { "" }, out)
}

// ------------------------------------------------------------ Careers ----
pub struct Career {
    pub name: &'static str,
    pub survival: (usize, i32),
    pub advancement: (usize, i32),
    /// Gained at level 0 in basic training.
    pub service: &'static [&'static str],
    /// Rolled on each term.
    pub table: &'static [&'static str],
    pub ranks: [&'static str; 7],
}

pub static CAREERS: &[Career] = &[
    Career {
        name: "Agent",
        survival: (INT, 6),
        advancement: (EDU, 6),
        service: &["Investigate", "Streetwise"],
        table: &["Gun Combat", "Gun Combat", "Melee Combat", "Melee Combat", "Persuade", "Comms",
                 "Recon", "Deception", "Computer", "Tactics", "Investigate", "Streetwise"],
        ranks: ["Agent", "Special Agent", "Senior Agent", "Agent in Charge",
                "Assistant Director", "Director", "Director General"],
    },
    Career {
        name: "Army",
        survival: (END, 5),
        advancement: (EDU, 7),
        service: &["Drive", "Athletics"],
        table: &["Gun Combat", "Gun Combat", "Melee Combat", "Recon", "Heavy Weapons", "Tactics",
                 "Leadership", "Survival", "Medic", "Stealth", "Drive", "Athletics"],
        ranks: ["Private", "Lance Corporal", "Corporal", "Sergeant", "Lieutenant", "Captain", "Colonel"],
    },
    Career {
        name: "Marine",
        survival: (END, 6),
        advancement: (EDU, 7),
        service: &["Vacc Suit", "Athletics"],
        table: &["Gun Combat", "Gun Combat", "Melee Combat", "Melee Combat", "Heavy Weapons", "Tactics",
                 "Leadership", "Recon", "Stealth", "Survival", "Vacc Suit", "Explosives"],
        ranks: ["Marine", "Lance Corporal", "Corporal", "Sergeant", "Lieutenant", "Captain", "Force Commander"],
    },
    Career {
        name: "Navy",
        survival: (INT, 5),
        advancement: (EDU, 6),
        service: &["Vacc Suit", "Mechanic"],
        table: &["Pilot", "Gunner", "Engineer", "Astrogation", "Electronics", "Vacc Suit",
                 "Leadership", "Tactics", "Mechanic", "Gun Combat", "Admin", "Comms"],
        ranks: ["Crewman", "Able Spacehand", "Petty Officer", "Ensign", "Lieutenant", "Commander", "Captain"],
    },
    Career {
        name: "Scout",
        survival: (END, 7),
        advancement: (INT, 9),
        service: &["Pilot", "Survival"],
        table: &["Pilot", "Pilot", "Astrogation", "Vacc Suit", "Survival", "Recon",
                 "Navigation", "Electronics", "Mechanic", "Gun Combat", "Comms", "Science"],
        ranks: ["Scout", "Courier", "Senior Scout", "Survey Lead", "Field Chief", "Sector Chief", "Director"],
    },
    Career {
        name: "Merchant",
        survival: (EDU, 5),
        advancement: (INT, 7),
        service: &["Broker", "Steward"],
        table: &["Broker", "Broker", "Persuade", "Admin", "Steward", "Pilot",
                 "Engineer", "Streetwise", "Deception", "Carouse", "Advocate", "Gun Combat"],
        ranks: ["Crewman", "Senior Crewman", "4th Officer", "3rd Officer", "2nd Officer", "1st Officer", "Captain"],
    },
    Career {
        name: "Rogue",
        survival: (DEX, 6),
        advancement: (INT, 7),
        service: &["Streetwise", "Deception"],
        table: &["Streetwise", "Deception", "Stealth", "Gun Combat", "Melee Combat", "Electronics",
                 "Persuade", "Gambler", "Recon", "Carouse", "Athletics", "Pilot"],
        ranks: ["Thug", "Enforcer", "Fixer", "Lieutenant", "Boss", "Underboss", "Kingpin"],
    },
    Career {
        name: "Noble",
        survival: (SOC, 5),
        advancement: (INT, 8),
        service: &["Diplomat", "Carouse"],
        table: &["Diplomat", "Persuade", "Leadership", "Admin", "Advocate", "Carouse",
                 "Gambler", "Art", "Language", "Melee Combat", "Deception", "Investigate"],
        ranks: ["Courtier", "Knight", "Baronet", "Baron", "Marquis", "Count", "Duke"],
    },
    Career {
        name: "Scholar",
        survival: (EDU, 5),
        advancement: (INT, 8),
        service: &["Science", "Computer"],
        table: &["Science", "Science", "Computer", "Medic", "Investigate", "Language",
                 "Electronics", "Admin", "Diplomat", "Comms", "Persuade", "Engineer"],
        ranks: ["Student", "Researcher", "Scientist", "Senior Scientist", "Professor", "Dean", "Chancellor"],
    },
    Career {
        name: "Entertainer",
        survival: (INT, 5),
        advancement: (DEX, 7),
        service: &["Art", "Carouse"],
        table: &["Art", "Art", "Carouse", "Persuade", "Deception", "Steward",
                 "Athletics", "Language", "Streetwise", "Gambler", "Comms", "Stealth"],
        ranks: ["Performer", "Featured Act", "Headliner", "Star", "Celebrity", "Icon", "Legend"],
    },
    Career {
        name: "Citizen",
        survival: (EDU, 5),
        advancement: (INT, 6),
        service: &["Admin", "Drive"],
        table: &["Admin", "Drive", "Mechanic", "Computer", "Trade", "Persuade",
                 "Steward", "Streetwise", "Broker", "Electronics", "Medic", "Flyer"],
        ranks: ["Worker", "Technician", "Supervisor", "Manager", "Senior Manager", "Director", "Executive"],
    },
    Career {
        name: "Drifter",
        survival: (END, 6),
        advancement: (INT, 7),
        service: &["Survival", "Streetwise"],
        table: &["Survival", "Streetwise", "Athletics", "Melee Combat", "Mechanic", "Recon",
                 "Stealth", "Gun Combat", "Deception", "Pilot", "Carouse", "Vacc Suit"],
        ranks: ["Wanderer", "Wanderer", "Survivor", "Survivor", "Veteran Drifter", "Veteran Drifter", "Legend of the Lanes"],
    },
];

// ------------------------------------------------------------ Species ----
pub const SPECIES: &[&str] = &["Human", "Solomani", "Vilani", "Zhodani", "Aslan", "Vargr", "Droyne"];

pub fn species_mods(species: &str) -> [i32; 6] {
    match species {
        "Aslan" => [2, -2, 0, 0, 0, 0],
        "Vargr" => [-1, 1, -1, 0, 0, 0],
        "Droyne" => [-2, 1, 0, 0, 0, 0],
        "Zhodani" => [0, 0, 0, 0, 0, 1],
        _ => [0; 6],
    }
}

// ---------------------------------------------------- Flavour tables ----
const FIRST: &[&str] = &["Mara", "Jonas", "Ilse", "Teodor", "Kiri", "Anya", "Rhys", "Sabine", "Dmitri",
    "Nadia", "Oskar", "Lena", "Cassius", "Yara", "Elias", "Veska", "Tomas", "Ines", "Hal", "Soren"];
const LAST: &[&str] = &["Vance", "Okonkwo", "Hollis", "Arkady", "Strand", "Kemal", "Duquesne", "Laine",
    "Rourke", "Tashkent", "Bryne", "Alvarez", "Mercer", "Iversen", "Sato", "Calder", "Novak", "Reyes"];

pub const TRAITS: &[&str] = &["Charming", "Paranoid", "Blunt", "Meticulous", "Reckless", "Soft-spoken",
    "Cynical", "Loyal to a fault", "Restless", "Superstitious", "Arrogant", "Dry wit", "Haunted",
    "Ambitious", "Generous", "Secretive"];
pub const WANTS: &[&str] = &["Escape an old employer", "Buy a ship of their own", "Clear their name",
    "Find a missing sibling", "Retire somewhere quiet", "Expose a corrupt official",
    "Settle a blood debt", "Recover a lost artefact", "Earn a noble title",
    "Protect a former partner", "Get off-world before the next election", "Make one last big score"];
pub const HOOKS: &[&str] = &["Owes money to a dangerous person", "Carries data someone will kill for",
    "Is being quietly watched by Imperial intelligence", "Has a bounty on a nearby world",
    "Knows where a derelict ship lies", "Is secretly working for a rival faction",
    "Was framed for a crime they didn't commit", "Holds a debt marker over a local official",
    "Has an ex-partner who wants them dead", "Is searching for a psionic institute"];
pub const QUOTES: &[&str] = &[
    "Some doors are best left open... and some debts are best forgotten.",
    "Everyone's honest until the jump fuel runs out.",
    "I don't do favours. I do arrangements.",
    "The stars don't care who you were. Neither do I.",
    "Trust the ship. Check the crew.",
    "Every port has a back door. I just know where to knock.",
    "Credits spend the same, whoever paid them.",
    "Survive first. Ask questions in jump space.",
];
const HUMAN_QUOTES: &[&str] = &[
    "Never trust a starport that's cheaper than the one before it.",
    "I've been shot at by professionals. The amateurs are worse.",
    "It's not a bribe. It's a 'handling fee for the paperwork gods'.",
    "My plan is simple: smile, nod, and keep one hand near the airlock.",
    "Jump drives and good intentions both fail at the worst moment.",
    "If it's cheap, it's stolen. If it's free, you're the cargo.",
    "I'm not lost. I'm taking the scenic route through three star systems.",
    "Retirement? Out here, that's just a long way of saying 'got away with it'.",
    "The Imperium runs on two things: jump fuel and forms in triplicate.",
    "Never name your ship after a hope. It hears you.",
];
const SOLOMANI_QUOTES: &[&str] = &[
    "Terra didn't conquer the stars by being polite about it.",
    "Mankind first, and second, and third. Then maybe a decent cup of coffee.",
    "The Imperium would be so much better run by people who actually come from Earth.",
    "Where I come from, 'Confederation' is spelled 'Solomani', and we only misspell it on purpose.",
    "I'm not a bigot. I just have very high standards for who gets the good seat.",
    "Admit it: everyone's ancestors came from Old Earth. Some just forgot to pack a receipt.",
    "My grandfather fought for the Rim. He'd be horrified at the state of the coffee out here.",
    "Humanity has a destiny. Mine is currently stuck in customs.",
    "Say what you like about Terra, nobody ever lost a war by being too stubborn about it. Oh wait.",
    "We have the best history in the galaxy. Ask us. We'll tell you. Slowly.",
];
const VILANI_QUOTES: &[&str] = &[
    "We have done it this way for five thousand years. Why would we stop on a Wednesday?",
    "Please take a number. Your number is four million, three hundred thousand.",
    "Efficiency is the art of filling out the right form in the right order, in the right ink.",
    "The Ziru Sirka endured for millennia. Surely a short delay of eleven weeks is nothing.",
    "Change is acceptable, provided it is approved by the Committee on Acceptable Change.",
    "Trade is the foundation of civilization. Trade tariffs are the foundation of my pension.",
    "One does not hurry a good bureaucrat. One merely waits, and brings tea.",
    "I do not dislike risk. I dislike it being unrecorded.",
    "In triplicate. And again in triplicate, in case the first triplicate gets ideas.",
    "The Vilani way: do nothing rash, and keep very careful minutes of it.",
];
const ZHODANI_QUOTES: &[&str] = &[
    "You are thinking about lying to me. Please do not. It is so tedious to correct.",
    "In the Consulate, we say that thoughts are private. We say it very politely, while reading them.",
    "My intuition tells me you'd like to rob me. It's also telling me how.",
    "Do not worry, I am only reading the surface thoughts. The surface is quite enough.",
    "Nobility is a duty. Telepathy is a privilege. Parking is a nuisance.",
    "We Zhodani value harmony. That's why we know when you are lying.",
    "Our Intendants prefer prevention to punishment. It is astonishing what you can prevent with advance warning.",
    "Please keep your thoughts to a reasonable volume. I have a headache.",
    "The Imperium calls it invasion. We call it a very long-overdue tour.",
    "I never raise my voice. I simply raise the subject you were hoping to avoid.",
];
const ASLAN_QUOTES: &[&str] = &[
    "Land is honor. Honor is land. The rest is conversation, and I did not come here to talk.",
    "I challenge you. Not for any reason. I simply like how you lose.",
    "A true male does not shop. He acquires.",
    "My ancestors conquered a system with a single claw and a very sharp opinion.",
    "You have insulted my clan. I will allow you to apologise while running.",
    "Ihatei, they call us — second sons. I prefer 'a man with unfinished business and a good claw'.",
    "Never argue with a female about money. You will lose the argument, the money, and the argument about the money.",
    "A feast without a fight is merely dinner. I am disappointed in this dinner.",
    "The Imperium holds land by treaty. We hold it by the throat. Both are valid.",
    "I would never lie to you. A lie is a coward's claws. I would simply dissuade you from asking.",
    "My mane took twenty years to grow and I will not discuss it with strangers.",
    "I submit to your strength. I will however be back with more of mine.",
];
const VARGR_QUOTES: &[&str] = &[
    "Follow me, follow me! I have an excellent plan and a very good feeling about it!",
    "A pack is only as strong as its loudest howler, and I am extremely loud.",
    "I don't have loyalty issues. I have leadership opportunities.",
    "The Imperium is a fine place. Very big. Plenty of throats to... shake hands with.",
    "Ask three Vargr for advice and you get four plans and a coup.",
    "My charisma is not a trick. My charisma is mostly teeth.",
    "I lead because someone must. And because the last leader is sleeping in the cargo hold.",
    "They say Vargr cannot be trusted. I trust most of me.",
    "We do not break promises. We simply... renegotiate, vigorously, at three in the morning.",
    "Do not mistake my smile for friendliness. It is how my face works.",
    "The best things in life are free. The second best are in someone else's cargo hold.",
];
const DROYNE_QUOTES: &[&str] = &[
    "The Sleepers have not told us everything. We would very much like to be told.",
    "Workers work. Warriors fight. Sports... sport. I am a diplomat; I mostly look worried.",
    "Chirp. Chirp-chirp. That is what I said and what I meant, in order.",
    "This one has a message from the Ancients. It is mostly about lunch.",
    "We do not understand you. You do not understand us. It is the basis of excellent relations.",
    "Please do not touch the crest. That is a very private ritual.",
    "The Caste is a harmony. If you have never felt harmony, it is like being slightly embarrassed.",
    "Seven small steps for the Drone. One large mystery for you.",
    "I am a Technician. I would fix your ship, but I would also like to know what you have been doing to it.",
    "Wings are not for flying. Wings are for being dramatic from a great height.",
];
const MISHAPS: &[&str] = &["severely injured in the line of duty", "betrayed by a colleague",
    "blamed for a disaster", "caught up in a scandal", "an operation went badly wrong",
    "forced out by a superior", "a rival ruined their reputation"];

// ---------------------------------------------------- Appearance data ----
fn pick_s(r: &mut Rng, items: &[&str]) -> String {
    r.pick(items).to_string()
}

/// Physical description fields; each is editable and feeds `Npc::image_prompt`.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "gui", derive(serde::Serialize, serde::Deserialize))]
pub struct Look {
    /// Species form, height and build.
    pub body: String,
    /// Skin, fur or scales.
    pub skin: String,
    /// Hair, mane, crest or head covering of fur.
    pub hair: String,
    pub eyes: String,
    /// Scars, tattoos, jewellery, other distinguishing details.
    pub mark: String,
    pub attire: String,
    pub expression: String,
    /// Background / setting for the picture.
    pub setting: String,
}

fn attire_for(career: &str, r: &mut Rng) -> String {
    let opts: &[&str] = match career {
        "Agent" => &["a charcoal tailored coat over a concealed armoured vest", "a rumpled suit with a shoulder holster and a cheap comm earpiece"],
        "Army" => &["worn combat fatigues with unit patches and a battered load-bearing harness", "a mottled field uniform, rolled sleeves, dog tags at the collar"],
        "Marine" => &["a scuffed ablative-plate vacc-armour torso piece over a dark skinsuit", "a sharply pressed dress uniform with a row of campaign ribbons"],
        "Navy" => &["a navy-blue ship's coverall with rank insignia and a fleet patch", "a crisp bridge-officer jacket with brass-edged collar tabs"],
        "Scout" => &["a patched, sun-faded scout jumpsuit with many pockets and a survey data slate on the hip", "a hard-wearing exploration jacket and a vacc-suit collar ring slung at the belt"],
        "Merchant" => &["a practical merchant-line jacket with the company crest, a cargo manifest slate tucked under one arm", "a well-cut ship's-officer coat with a ledger-stylus clipped at the chest"],
        "Rogue" => &["a dark leather duster with high collar and hidden pockets", "a flashy but scuffed jacket, rings on several fingers, a knife handle visible at the boot"],
        "Noble" => &["an embroidered high-collared tunic of fine fabric with a house signet ring", "a tailored formal coat in deep colours, with a heraldic brooch"],
        "Scholar" => &["a long academic coat, ink-stained cuffs, and a pair of data-lenses pushed up on the forehead", "a plain tunic with a lab smock, a stylus behind the ear"],
        "Entertainer" => &["a theatrical, richly coloured stage costume with metallic thread", "stylish layered clothing in bold colours, a glittering sash"],
        "Citizen" => &["a clean practical work jacket with a company badge", "plain functional civilian clothes, sleeves pushed up, a tool pouch at the belt"],
        _ => &["mismatched salvaged clothing, a heavy travelling cloak, and a worn pack strap across the chest", "a threadbare spacer's jacket patched in several places, fingerless gloves"],
    };
    r.pick(opts).to_string()
}

fn setting_for(career: &str, r: &mut Rng) -> String {
    let opts: &[&str] = match career {
        "Agent" => &["a rain-slicked starport street at night, neon reflections", "a dim safehouse office with drawn blinds"],
        "Army" | "Marine" => &["a dusty military staging ground under a hazy alien sky", "the cramped hold of a troop transport, red emergency lighting"],
        "Navy" | "Scout" => &["the cluttered bridge of a starship with glowing holo-displays", "a ship's corridor lit by cool blue strip lights"],
        "Merchant" => &["a busy cargo bay with stacked containers and a loading drone", "a starport trading floor crowded with merchants"],
        "Noble" => &["a grand hall with tall arched windows and heraldic banners", "a private balcony above a glittering orbital city"],
        "Scholar" => &["a cluttered laboratory with holographic star charts", "a quiet library alcove lit by a warm lamp"],
        "Entertainer" => &["backstage of a theatre, mirrors and soft spotlights", "a smoky lounge with coloured lights"],
        _ => &["a noisy starport bar, warm amber light and smoke", "the edge of a landing pad at dusk, a battered freighter behind"],
    };
    r.pick(opts).to_string()
}

fn look_for(r: &mut Rng, species: &str, career: &str, stats: &[i32; 6], age: u32, injured: bool, trait_: &str) -> Look {
    let (str_, dex, end, soc) = (stats[STR], stats[DEX], stats[END], stats[SOC]);
    let build_word = match (str_ + end, dex) {
        (s, _) if s >= 20 => "powerfully built, broad-shouldered",
        (s, d) if s >= 14 && d >= 9 => "athletic and lean-muscled",
        (s, _) if s <= 8 => "slight and wiry",
        (_, d) if d >= 11 => "lithe and quick-looking",
        _ => "average build",
    };
    let age_word = match age {
        0..=25 => "youthful",
        26..=33 => "in their prime",
        34..=41 => "weathered by years of service",
        _ => "grizzled and visibly ageing",
    };
    let groomed = if soc >= 10 { "impeccably groomed" } else if soc <= 4 { "unkempt" } else { "tidy enough" };

    let mut look = Look::default();
    match species {
        "Aslan" => {
            look.body = format!("tall leonine Aslan humanoid, {build_word}, {age_word}, digitigrade stance, retractable claws, dewclaws on the wrists");
            look.skin = format!("short {} fur with darker markings along the spine and forearms, {groomed}",
                r.pick(&["tawny gold", "russet", "dark brown", "sandy grey", "tawny with black tipping", "pale cream"]));
            look.hair = format!("a thick mane worn {}", r.pick(&["long and loose", "braided with small brass clan beads", "swept back and oiled", "cropped short and ragged", "tied in a warrior's knot"]));
            look.eyes = format!("{} slit-pupilled eyes under a heavy brow", r.pick(&["amber", "green-gold", "pale yellow", "deep brown", "copper"]));
            look.mark = pick_s(r, &["old claw scars across the muzzle", "a notched ear from a duel", "a gold clan ring through one ear", "a small clan sigil shaved into the mane", "jewelled bands on both wrists", "a faded scar across the chest fur"]);
        }
        "Vargr" => {
            look.body = format!("bipedal Vargr, a canine humanoid with a long muzzle, {build_word}, {age_word}, slim digitigrade legs");
            look.skin = format!("a {} coat of short fur, {groomed}", r.pick(&["grey wolf", "black", "ruddy brown", "pale sand", "brindled grey-brown", "white with dark ears"]));
            look.hair = format!("a crest of longer fur along the neck, {}", r.pick(&["worn loose", "slicked back", "dyed with a bold stripe", "ragged and matted"]));
            look.eyes = format!("{} eyes, alert and restless", r.pick(&["amber", "pale blue", "green", "dark brown"]));
            look.mark = pick_s(r, &["torn ears with metal rings", "a patchy scar along the muzzle", "a fine braided cord with trinkets on one wrist", "a shaved patch over an old wound", "a bandolier of small charms"]);
        }
        "Droyne" => {
            look.body = format!("small Droyne, a reptilian-avian humanoid about 1.2 m tall, {build_word}, {age_word}, with four limbs plus folded leathery wings");
            look.skin = format!("fine {} scales, {groomed}", r.pick(&["olive-green", "slate-grey", "russet", "pale bronze", "mottled blue-green"]));
            look.hair = format!("a bony crest rising from the skull, {}", r.pick(&["bright red", "mustard yellow", "deep blue", "banded black and white"]));
            look.eyes = format!("large, round {} eyes with a nictitating membrane", r.pick(&["black", "amber", "silver", "violet"]));
            look.mark = pick_s(r, &["a leather harness carrying small tools", "tiny coloured cords knotted around the crest", "a chipped wing-edge", "ritual paint stripes on the cheeks"]);
        }
        _ => {
            let height = match (species, str_) {
                ("Zhodani", _) => "tall and slender",
                (_, s) if s >= 10 => "tall",
                (_, s) if s <= 5 => "short",
                _ => "average height",
            };
            look.body = format!("{} human, {height}, {build_word}, {age_word}", if species == "Human" { "adult" } else { species });
            look.skin = format!("{} skin, {groomed}", r.pick(&["fair", "olive", "light brown", "deep brown", "tanned", "pale", "warm bronze"]));
            let colour = *r.pick(&["black", "dark brown", "chestnut", "auburn", "blond", "sandy", "silver-streaked", "copper"]);
            let style = *r.pick(&["cropped short", "long and tied back", "shaved at the sides", "wavy and shoulder-length", "tightly braided", "close-cut with a stubble beard", "swept back with a neat moustache", "a bald head"]);
            look.hair = if style == "a bald head" { style.to_string() } else { format!("{colour} hair, {style}") };
            look.eyes = format!("{} eyes", r.pick(&["grey", "dark brown", "green", "hazel", "pale blue", "amber", "steel-blue"]));
            look.mark = pick_s(r, &["a thin scar through one eyebrow", "a small tattoo on the neck", "a cybernetic implant behind one ear", "a crooked, once-broken nose", "faint laugh lines and a watchful squint", "a silver earring and a worn wristwatch", "burn scars across the back of one hand"]);
            if species == "Zhodani" {
                look.mark += ", elegant restrained dress in muted tones";
            }
        }
    }
    if injured {
        look.mark += &format!(", plus {}", r.pick(&["a long healed scar down the side of the face", "a prosthetic hand", "a pronounced limp-hinting posture", "a cybernetic eye with a faint glow"]));
    }
    look.attire = attire_for(career, r);
    look.expression = format!("{} expression, with a {} air about them",
        r.pick(&["guarded", "intense", "wry", "tired but steady", "calm", "sharp", "amused"]),
        trait_.to_lowercase());
    look.setting = setting_for(career, r);
    look
}

// --------------------------------------------------------- NPC model ----
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "gui", derive(serde::Serialize, serde::Deserialize))]
pub enum StatMethod {
    TwoD6,
    ThreeD6DropLowest,
    Heroic,
}

impl StatMethod {
    pub const ALL: [StatMethod; 3] = [StatMethod::TwoD6, StatMethod::ThreeD6DropLowest, StatMethod::Heroic];
    pub fn label(self) -> &'static str {
        match self {
            StatMethod::TwoD6 => "2D6 (standard)",
            StatMethod::ThreeD6DropLowest => "3D6 drop lowest",
            StatMethod::Heroic => "Heroic (2D6+2, max 15)",
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "gui", derive(serde::Serialize, serde::Deserialize))]
pub struct GenOptions {
    pub name: String,
    /// None = random
    pub species: Option<usize>,
    /// None = random
    pub career: Option<usize>,
    /// None = random 1..=7
    pub terms: Option<u32>,
    pub method: StatMethod,
    pub aging: bool,
    pub end_on_mishap: bool,
    pub seed: u64,
    pub lock_seed: bool,
    pub portrait_path: String,
}

impl Default for GenOptions {
    fn default() -> Self {
        GenOptions {
            name: String::new(),
            species: Some(0),
            career: None,
            terms: None,
            method: StatMethod::TwoD6,
            aging: true,
            end_on_mishap: true,
            seed: 1,
            lock_seed: false,
            portrait_path: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "gui", derive(serde::Serialize, serde::Deserialize))]
pub struct TermLog {
    pub term: u32,
    pub age_start: u32,
    pub age_end: u32,
    pub events: Vec<String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "gui", derive(serde::Serialize, serde::Deserialize))]
pub struct Npc {
    pub name: String,
    pub species: String,
    pub career: String,
    pub rank: usize,
    pub rank_title: String,
    pub age: u32,
    pub terms: u32,
    pub stats: [i32; 6],
    pub skills: BTreeMap<String, i32>,
    pub log: Vec<TermLog>,
    pub credits: i64,
    pub trait_: String,
    pub wants: String,
    pub hook: String,
    pub quote: String,
    pub seed: u64,
    pub portrait_path: String,
    #[cfg_attr(feature = "gui", serde(default))]
    pub look: Look,
}

impl Npc {
    pub fn upp(&self) -> String {
        self.stats.iter().map(|&v| ehex(v)).collect()
    }
    pub fn title(&self) -> String {
        format!("{} {}", self.species, self.rank_title).to_uppercase()
    }
    pub fn muster_line(&self) -> String {
        format!(
            "Mustered out at age {} after {} term{} with {}.",
            self.age,
            self.terms,
            if self.terms == 1 { "" } else { "s" },
            format_credits(self.credits)
        )
    }

    /// Reroll only the roleplaying details (trait / wants / hook / quote).
    /// Quote pool for a species (humans fall back to the generic list).
    fn species_quotes(species: &str) -> &'static [&'static str] {
        match species {
            "Solomani" => SOLOMANI_QUOTES,
            "Vilani" => VILANI_QUOTES,
            "Zhodani" => ZHODANI_QUOTES,
            "Aslan" => ASLAN_QUOTES,
            "Vargr" => VARGR_QUOTES,
            "Droyne" => DROYNE_QUOTES,
            _ => HUMAN_QUOTES,
        }
    }

    pub fn reroll_roleplay(&mut self, seed: u64) {
        let mut r = Rng::new(seed);
        self.trait_ = r.pick(TRAITS).to_string();
        self.wants = r.pick(WANTS).to_string();
        self.hook = r.pick(HOOKS).to_string();
        // Mostly species-flavoured quotes, with the odd generic spacer line.
        let pool = if r.below(4) == 0 { QUOTES } else { Self::species_quotes(&self.species) };
        self.quote = r.pick(pool).to_string();
    }

    /// Reroll only the physical description.
    pub fn reroll_look(&mut self, seed: u64) {
        let mut r = Rng::new(seed);
        let injured = self.log.iter().any(|t| t.events.iter().any(|e| e.starts_with("Survival failed")));
        self.look = look_for(&mut r, &self.species, &self.career, &self.stats, self.age, injured, &self.trait_);
    }

    /// Text-to-image prompt assembled from the editable `Look` fields.
    pub fn image_prompt(&self, style: &str) -> String {
        let l = &self.look;
        let style = if style.trim().is_empty() { "Cinematic science-fiction character portrait" } else { style.trim() };
        format!(
            "{style}. Three-quarter portrait of {name}, a {age}-year-old {career} ({rank}) in a gritty \
far-future space-opera setting. Physique: {body}. Skin/fur: {skin}. Hair/mane: {hair}. Eyes: {eyes}. \
Distinguishing features: {mark}. Wearing {attire}. Expression: {expr}. Background: {setting}. \
Moody directional lighting with rim light, shallow depth of field, richly detailed textures, \
vertical 3:4 composition. No text, no captions, no watermark, no extra limbs or fingers.",
            name = self.name, age = self.age, career = self.career.to_lowercase(),
            rank = self.rank_title.to_lowercase(), body = l.body, skin = l.skin, hair = l.hair,
            eyes = l.eyes, mark = l.mark, attire = l.attire, expr = l.expression, setting = l.setting,
        )
    }

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s += &format!("# {}\n\n", self.title());
        s += &format!("**{}** — {}, age {}, {} terms\n\n", self.name, self.career, self.age, self.terms);
        s += &format!("UPP **{}** · {}\n\n", self.upp(), format_credits(self.credits));
        s += "| Stat | Value | DM |\n|---|---|---|\n";
        for (i, n) in STAT_NAMES.iter().enumerate() {
            s += &format!("| {} | {} | {} |\n", n, self.stats[i], format_dm(stat_dm(self.stats[i])));
        }
        s += "\n## Skills\n\n";
        for (k, v) in &self.skills {
            s += &format!("- {k} {v}\n");
        }
        s += "\n## Career log\n\n";
        for t in &self.log {
            s += &format!("**Term {}** (age {}–{})\n", t.term, t.age_start, t.age_end);
            for e in &t.events {
                s += &format!("- {e}\n");
            }
            s += "\n";
        }
        s += &format!("{}\n\n## Roleplaying\n\n", self.muster_line());
        s += &format!("- **Trait:** {}\n- **Wants:** {}\n- **Hook:** {}\n\n> \u{201c}{}\u{201d}\n",
            self.trait_, self.wants, self.hook, self.quote);
        s += &format!("\n## Appearance\n\n{}\n", self.image_prompt(""));
        s += &format!("\n_Seed {}_\n", self.seed);
        s
    }
}

// --------------------------------------------------------- Generator ----
fn roll_stat(r: &mut Rng, m: StatMethod) -> i32 {
    match m {
        StatMethod::TwoD6 => r.roll(2),
        StatMethod::ThreeD6DropLowest => {
            let mut d = [r.d6(), r.d6(), r.d6()];
            d.sort();
            d[1] + d[2]
        }
        StatMethod::Heroic => (r.roll(2) + 2).min(15),
    }
}

fn gain_skill(r: &mut Rng, career: &Career, skills: &mut BTreeMap<String, i32>) -> String {
    // Try a few times to avoid pushing a skill past 4.
    for _ in 0..8 {
        let name = *r.pick(career.table);
        let lvl = skills.entry(name.to_string()).or_insert(0);
        if *lvl < 4 {
            *lvl += 1;
            return format!("{name} {}", *lvl);
        }
    }
    let name = *r.pick(career.table);
    format!("{name} {}", skills.get(name).copied().unwrap_or(0))
}

pub fn generate(opts: &GenOptions) -> Npc {
    let mut r = Rng::new(opts.seed);

    let species = match opts.species {
        Some(i) => SPECIES[i.min(SPECIES.len() - 1)],
        None => *r.pick(SPECIES),
    };
    let career_idx = opts.career.unwrap_or_else(|| r.below(CAREERS.len())).min(CAREERS.len() - 1);
    let career = &CAREERS[career_idx];
    let max_terms = opts.terms.unwrap_or_else(|| r.below(7) as u32 + 1).clamp(1, 12);

    let mods = species_mods(species);
    let mut stats = [0; 6];
    for i in 0..6 {
        stats[i] = (roll_stat(&mut r, opts.method) + mods[i]).max(1);
    }

    // Aslan get Trokh-derived names; other species keep their existing RNG stream.
    let name = if !opts.name.trim().is_empty() {
        opts.name.trim().to_string()
    } else if species == "Aslan" {
        crate::names::aslan_name(&mut r)
    } else {
        format!("{} {}", r.pick(FIRST), r.pick(LAST))
    };

    let mut skills: BTreeMap<String, i32> = BTreeMap::new();
    let mut log = Vec::new();
    let mut rank = 0usize;
    let mut age = 18u32;
    let mut terms_done = 0u32;

    for t in 1..=max_terms {
        let mut ev = Vec::new();
        let age_start = age;
        if t == 1 {
            ev.push("Basic training: all service skills at 0".to_string());
            for s in career.service {
                skills.entry(s.to_string()).or_insert(0);
            }
        }

        let (ss, st) = career.survival;
        let roll = r.roll(2) + stat_dm(stats[ss]);
        let survived = roll >= st;
        if survived {
            ev.push(format!("Survived ({} {} vs {}+)", STAT_NAMES[ss], roll, st));
            let (as_, at) = career.advancement;
            let aroll = r.roll(2) + stat_dm(stats[as_]);
            if aroll >= at && rank < 6 {
                rank += 1;
                let verb = if rank == 1 { "Commissioned" } else { "Promoted" };
                ev.push(format!("{verb} ({} {} vs {}+): {}", STAT_NAMES[as_], aroll, at, career.ranks[rank]));
            } else {
                let what = if rank == 0 { "Commission" } else { "Promotion" };
                ev.push(format!("{what} failed ({} {} vs {}+)", STAT_NAMES[as_], aroll, at));
            }
            let a = gain_skill(&mut r, career, &mut skills);
            let b = gain_skill(&mut r, career, &mut skills);
            ev.push(format!("Gained: {a}, {b}"));
        } else {
            ev.push(format!("Survival failed ({} {} vs {}+): {}", STAT_NAMES[ss], roll, st, r.pick(MISHAPS)));
            let a = gain_skill(&mut r, career, &mut skills);
            ev.push(format!("Gained: {a}"));
        }

        age += 4;
        terms_done = t;

        if opts.aging && t >= 4 {
            let aroll = r.roll(2) - t as i32;
            if aroll >= 1 {
                ev.push(format!("Aging check passed ({aroll})"));
            } else {
                let phys = [STR, DEX, END];
                let s = *r.pick(&phys);
                let loss = if aroll <= -3 { 2 } else { 1 };
                stats[s] = (stats[s] - loss).max(1);
                ev.push(format!("Aging check failed ({aroll}): {} -{loss}", STAT_NAMES[s]));
            }
        }

        log.push(TermLog { term: t, age_start, age_end: age, events: ev });
        if !survived && opts.end_on_mishap {
            break;
        }
    }

    // Mustering out: one cash roll per term, max 3; +1 DM at rank 5+.
    const CASH: [i64; 7] = [1_000, 2_000, 5_000, 5_000, 10_000, 20_000, 50_000];
    let mut credits = 0i64;
    for _ in 0..terms_done.min(3) {
        let idx = (r.d6() - 1 + if rank >= 5 { 1 } else { 0 }) as usize;
        credits += CASH[idx.min(6)];
    }

    let mut npc = Npc {
        name,
        species: species.to_string(),
        career: career.name.to_string(),
        rank,
        rank_title: career.ranks[rank].to_string(),
        age,
        terms: terms_done,
        stats,
        skills,
        log,
        credits,
        trait_: String::new(),
        wants: String::new(),
        hook: String::new(),
        quote: String::new(),
        seed: opts.seed,
        portrait_path: opts.portrait_path.clone(),
        look: Look::default(),
    };
    npc.reroll_roleplay(r.next_u64());
    npc.reroll_look(r.next_u64());
    npc
}

// -------------------------------------------------------------- Tests ----
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_npc() {
        let o = GenOptions { seed: 1234, ..Default::default() };
        let a = generate(&o);
        let b = generate(&o);
        assert_eq!(a.upp(), b.upp());
        assert_eq!(a.to_markdown(), b.to_markdown());
    }

    #[test]
    fn many_npcs_are_sane() {
        for seed in 0..2000 {
            for c in 0..CAREERS.len() {
                let o = GenOptions { seed, career: Some(c), species: None, ..Default::default() };
                let n = generate(&o);
                assert!(n.terms >= 1 && n.terms <= 7);
                assert_eq!(n.age, 18 + 4 * n.terms);
                assert_eq!(n.log.len() as u32, n.terms);
                assert!(n.stats.iter().all(|&v| v >= 1));
                assert!(n.skills.values().all(|&v| (0..=4).contains(&v)));
                assert_eq!(n.upp().len(), 6);
            }
        }
    }

    #[test]
    fn formatting() {
        assert_eq!(format_credits(17_000), "Cr17,000");
        assert_eq!(format_credits(1_250_000), "Cr1,250,000");
        assert_eq!(ehex(11), 'B');
        assert_eq!(ehex(18), 'J');
        assert_eq!(format_dm(stat_dm(12)), "+2");
        assert_eq!(format_dm(stat_dm(5)), "-1");
    }

    #[test]
    fn print_sample() {
        let o = GenOptions { seed: 7, career: Some(0), terms: Some(5), ..Default::default() };
        println!("{}", generate(&o).to_markdown());
    }
}
