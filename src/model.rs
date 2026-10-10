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
        "Agent" => &[
            "a charcoal tailored coat over a concealed armoured vest",
            "a rumpled suit with a shoulder holster and a cheap comm earpiece",
            "a discreet civilian jacket over a lightweight ballistic undershirt, with a false identity card clipped to the lapel",
            "an immaculate corporate suit, polished shoes, and a discreet encrypted communicator",
            "a weatherproof infiltration suit in muted colours, with lockpicks and surveillance tools hidden in its seams",
            "a fashionable off-world outfit, mirrored spectacles, and a forged starport access badge",
            "a worn intelligence-service field coat with reinforced elbows and an inconspicuous weapon holster",
            "a simple traveler's clothes concealing a compact sensor suite and a concealed sidearm",
            "a formal diplomatic suit with a discreet earpiece and an expression that reveals nothing",
            "a maintenance worker's coveralls, stained with machine grease and marked with a forged contractor logo",
            "a black tactical bodysuit beneath civilian streetwear, with a neural interface port behind one ear",
            "a civilian trench coat, worn gloves, and a battered briefcase secured with a biometric lock",
        ],
        "Army" => &[
            "worn combat fatigues with unit patches and a battered load-bearing harness",
            "a mottled field uniform, rolled sleeves, dog tags at the collar",
            "a heavy infantry uniform with reinforced knee pads and faded squad markings",
            "dust-covered combat fatigues, a utility belt, and a field dressing wrapped around one wrist",
            "a practical off-duty uniform with polished boots and a regimental insignia",
            "a desert-pattern combat suit with a battered helmet clipped to the backpack",
            "an armoured infantry undersuit with scratched plates and a heavily used rifle sling",
            "a military greatcoat over standard fatigues, its insignia almost worn away",
            "a lightweight recon uniform with camouflage panels and a compact field radio",
            "a formal army dress uniform with polished buttons, service ribbons, and immaculate boots",
            "a patched veteran's uniform with an old unit insignia sewn over a damaged shoulder",
            "a field engineer's uniform with a tool harness, diagnostic gloves, and a battered helmet",
        ],
        "Marine" => &[
            "a scuffed ablative-plate vacc-armour torso piece over a dark skinsuit",
            "a sharply pressed dress uniform with a row of campaign ribbons",
            "a full combat vacc-suit with magnetic boot soles and a scratched faceplate tucked under one arm",
            "a black marine undersuit with reinforced joints and a heavy-duty survival belt",
            "a ceremonial marine uniform with polished insignia and a sword-like parade sidearm",
            "heavy boarding armour with scarred shoulder plates and a compact breaching tool",
            "a sleeveless off-duty shirt, marine trousers, and a service tattoo visible on one arm",
            "a sealed assault suit with warning labels, an oxygen connection at the collar, and patched armour",
            "a decorated veteran's uniform, its medals contrasting with a badly scarred face",
            "a grey boarding-team uniform with a breaching harness and magnetic clamps",
            "a marine officer's tunic beneath a half-fastened tactical harness",
            "a bulky combat suit with reinforced gloves and an old unit emblem stencilled onto the chest",
        ],
        "Navy" => &[
            "a navy-blue ship's coverall with rank insignia and a fleet patch",
            "a crisp bridge-officer jacket with brass-edged collar tabs",
            "a well-worn duty uniform with rolled sleeves and a command badge at the shoulder",
            "a senior officer's dark formal tunic with gold piping and carefully polished boots",
            "a technical specialist's coveralls with diagnostic tools clipped to the chest",
            "a flight-deck uniform with coloured identification panels and a battered comm headset",
            "a spotless white ceremonial uniform with a high collar and silver rank bars",
            "a duty jacket thrown over a pressure-rated undersuit, with the helmet ring visible at the neck",
            "a navigation officer's uniform with a holographic star chart projected from a wrist console",
            "an engineering department uniform with burn marks on the sleeves and coolant stains at the cuffs",
            "a comfortable off-duty shipboard outfit with a small fleet insignia pinned near the collar",
            "a ship captain's tailored black uniform with restrained decorations and a signet-like command badge",
        ],
        "Scout" => &[
            "a patched, sun-faded scout jumpsuit with many pockets and a survey data slate on the hip",
            "a hard-wearing exploration jacket and a vacc-suit collar ring slung at the belt",
            "a weather-beaten environmental suit with a folded sensor mast strapped to the backpack",
            "a lightweight flight suit with pressure seals, worn gloves, and a scratched survey badge",
            "a faded field jacket over layered thermal clothing, with a portable survival kit at the waist",
            "a dusty surveyor's jumpsuit with geological samples secured in transparent tubes",
            "a compact vacc-suit with a battered helmet and a homemade patch covering an old puncture",
            "an expedition coat with reinforced shoulders, a climbing harness, and a battered map projector",
            "a lightweight scout uniform with a wrist-mounted scanner and boots stained by alien soil",
            "a patched cold-weather suit with insulated gloves and a compact portable heater",
            "a civilian spacer outfit with a scout-service insignia sewn discreetly inside the collar",
            "an old flight jacket, pressure-rated trousers, and a portable drone controller clipped to the belt",
        ],
        "Merchant" => &[
            "a practical merchant-line jacket with the company crest, a cargo manifest slate tucked under one arm",
            "a well-cut ship's-officer coat with a ledger-stylus clipped at the chest",
            "a rumpled cargo supervisor's uniform with a barcode scanner and a ring of access keys",
            "a polished commercial captain's coat with discreet corporate insignia and expensive boots",
            "a reinforced dockworker's jumpsuit with a loading harness and magnetic safety boots",
            "a brightly coloured trade representative's suit with several corporate badges",
            "a shipboard engineer's coveralls with a company patch and grease-stained gloves",
            "a conservative business suit with an expensive wrist terminal and a discreet translator",
            "a freighter crew uniform with repaired elbows and a faded ship's name over the breast pocket",
            "a cargo broker's long coat, practical boots, and a case full of contracts and shipping permits",
            "a luxury liner purser's uniform with gold trim and an immaculate service badge",
            "a free trader's battered flight jacket decorated with route patches from distant worlds",
        ],
        "Rogue" => &[
            "a dark leather duster with high collar and hidden pockets",
            "a flashy but scuffed jacket, rings on several fingers, a knife handle visible at the boot",
            "a sleeveless utility vest over a reinforced shirt, with suspicious bulges beneath the fabric",
            "a rumpled spacer's coat with mismatched fasteners and a stolen security badge",
            "a bright synthetic-fibre jacket, oversized boots, and a collection of questionable souvenirs",
            "a dark hooded travel outfit with a mirrored face shield and gloves concealing the fingerprints",
            "an expensive suit worn carelessly, its immaculate finish spoiled by a fresh bloodstain",
            "a battered flight jacket with contraband compartments stitched into the lining",
            "a streetwise outfit of layered fabrics, heavy boots, and a modified comm unit",
            "a patched vacc-suit with unofficial modifications and faded warning symbols",
            "a sleeveless leather coat, a heavy utility belt, and several old prison tattoos",
            "a flamboyant long coat, tinted glasses, and a holster worn just a little too visibly",
        ],
        "Noble" => &[
            "an embroidered high-collared tunic of fine fabric with a house signet ring",
            "a tailored formal coat in deep colours, with a heraldic brooch",
            "a luxurious silk jacket with subtle metallic embroidery and a discreet family crest",
            "a military-style court uniform with ceremonial decorations and polished boots",
            "an elegant evening outfit with a high collar, expensive jewellery, and immaculate gloves",
            "a riding-style coat of fine leather with an antique duelling pistol at the hip",
            "a traditional planetary costume adapted with modern fabrics and a personal communicator",
            "a severe black formal suit with a family signet and understated cufflinks",
            "a richly coloured diplomatic uniform with an ornate sash and carefully arranged medals",
            "a relaxed but unmistakably expensive outfit, with tailored trousers and a jewelled wrist terminal",
            "an elaborate court dress with a layered cape and heraldic motifs embroidered along the hem",
            "a minimalist designer suit whose understated cut suggests extraordinary expense",
        ],
        "Scholar" => &[
            "a long academic coat, ink-stained cuffs, and a pair of data-lenses pushed up on the forehead",
            "a plain tunic with a lab smock, a stylus behind the ear",
            "a rumpled researcher's jacket with specimen containers and a pocket full of handwritten notes",
            "a sterile laboratory coat over a high-tech environmental undersuit",
            "a comfortable academic outfit with a university insignia and a tablet full of research notes",
            "a field anthropologist's clothes with a shoulder bag full of recording equipment",
            "a formal lecturer's jacket, neatly combed hair, and an antique stylus used as a pointer",
            "a stained technical smock with insulated gloves and a portable diagnostic instrument",
            "an archaeologist's weathered expedition clothing with a brush and sample cases on the belt",
            "a heavy knitted cardigan over plain clothes, with several translucent data screens hovering nearby",
            "a practical laboratory jumpsuit with a transparent pocket full of labelled vials",
            "a dishevelled researcher outfit, with several mismatched data devices clipped to the belt",
        ],
        "Entertainer" => &[
            "a theatrical, richly coloured stage costume with metallic thread",
            "stylish layered clothing in bold colours, a glittering sash",
            "a sleek performance outfit with luminous piping and a programmable holographic cape",
            "a carefully distressed artist's jacket, paint-stained trousers, and a collection of unusual jewellery",
            "an extravagant velvet coat with oversized cuffs and a dramatic high collar",
            "a minimalist celebrity outfit with mirrored fabric and designer boots",
            "a traditional alien-inspired costume adapted for human audiences",
            "a dancer's flexible performance clothing with decorative light strips along the arms",
            "a polished lounge singer's suit with a jewelled microphone clipped to the lapel",
            "a street performer's layered clothes, battered instrument case, and a collection of tips",
            "a stage magician's tailored coat with hidden compartments and silver-threaded gloves",
            "an avant-garde outfit with asymmetrical panels, theatrical makeup, and a programmable colour-shifting sash",
        ],
        "Citizen" => &[
            "a clean practical work jacket with a company badge",
            "plain functional civilian clothes, sleeves pushed up, a tool pouch at the belt",
            "a factory worker's coveralls with reinforced knees and a compact diagnostic scanner",
            "a neat office outfit with an access badge and a wrist terminal",
            "a stained mechanic's jumpsuit with oil marks and insulated work gloves",
            "a practical medical uniform with a compact trauma kit and identification badge",
            "a delivery worker's jacket with reflective strips and a handheld parcel scanner",
            "comfortable casual clothes with a local sports team insignia and worn trainers",
            "a miner's heavy workwear with dust filters clipped to the collar",
            "a hospitality worker's smart uniform with a service badge and a folded towel over one arm",
            "a construction worker's reinforced trousers, tool harness, and scuffed protective boots",
            "a simple commuter outfit with a weatherproof coat and an inexpensive personal communicator",
        ],
        _ => &[
            "mismatched salvaged clothing, a heavy travelling cloak, and a worn pack strap across the chest",
            "a threadbare spacer's jacket patched in several places, fingerless gloves",
            "a battered environmental suit covered in faded planetary warning labels",
            "a weather-beaten coat over layered clothing, with a battered survival pack",
            "a collection of second-hand garments from several different worlds",
            "patched work clothes, reinforced boots, and an old military blanket slung over one shoulder",
            "a faded pressure suit with mismatched seals and a helmet held together with repair tape",
            "a practical travel outfit with a worn satchel full of tools and personal keepsakes",
            "a heavy insulated coat, scratched goggles, and gloves stained with machine oil",
            "a faded uniform stripped of its original insignia and repaired with mismatched fabric",
            "a dusty tunic beneath a scavenged armoured vest, with several items tied to a cord belt",
            "an anonymous grey outfit, well-worn boots, and a battered bag secured with wire",
        ],
    };
    r.pick(opts).to_string()
}

fn setting_for(career: &str, r: &mut Rng) -> String {
    let opts: &[&str] = match career {
        "Agent" => &[
            "a rain-slicked starport street at night, neon reflections",
            "a dim safehouse office with drawn blinds",
            "a crowded orbital concourse, departures scrolling across enormous holographic displays",
            "a sterile corporate interrogation room with a one-way observation panel",
            "a luxurious hotel suite overlooking the traffic lanes of a major orbital station",
            "a back alley beneath a flickering advertisement, delivery drones passing overhead",
            "a surveillance centre filled with camera feeds and silent intelligence analysts",
            "an embassy reception where diplomats and security personnel mingle beneath chandeliers",
            "a cramped maintenance passage behind the public areas of a starport",
            "a private club with mirrored walls, discreet security cameras, and expensive drinks",
            "a deserted industrial platform with distant ship engines glowing through the haze",
            "a crowded public market where rival agents could pass within arm's reach",
            "a small interrogation room lit by a single ceiling panel, with a recorder on the table",
            "a high-security archive filled with locked data terminals and biometric scanners",
        ],
        "Army" => &[
            "a dusty military staging ground under a hazy alien sky",
            "the cramped hold of a troop transport, red emergency lighting",
            "a muddy forward operating base surrounded by portable barriers",
            "a fortified checkpoint overlooking a sprawling industrial settlement",
            "a rain-soaked training ground with obstacle courses and armoured personnel carriers",
            "a field command post filled with tactical displays and exhausted officers",
            "a remote desert outpost beneath a sky crowded with orbital traffic",
            "a damaged barracks with overturned bunks and equipment drying near heaters",
            "an open military parade ground with regimental banners and inspection lines",
            "a frozen landscape dotted with sensor masts and armoured patrol vehicles",
            "a military vehicle depot filled with repair gantries and half-disassembled grav tanks",
            "a field hospital tent with medics, stretchers, and the distant sound of artillery",
            "a rugged mountain pass watched by automated sentry turrets",
            "an abandoned battlefield where wrecked drones lie half-buried in the dust",
        ],
        "Marine" => &[
            "a dusty military staging ground under a hazy alien sky",
            "the cramped hold of a troop transport, red emergency lighting",
            "a starship boarding bay with armoured marines preparing for a breach",
            "an illuminated assault shuttle hangar with mag-clamps and rows of combat armour",
            "a low-gravity urban battlefield with shattered windows and drifting debris",
            "the interior of a captured vessel, emergency lights reflecting off scorched bulkheads",
            "a shipboard armoury lined with vacc-suits, rifles, and sealed equipment lockers",
            "a ceremonial military hall displaying unit banners and battle honours",
            "a fortified embassy entrance surrounded by tactical barriers and security drones",
            "a planetary drop zone where assault capsules smoulder in the surrounding soil",
            "a pressurised corridor aboard a warship, hazard stripes leading toward a sealed hatch",
            "a marine training arena with holographic cover and automated target drones",
            "a ruined industrial complex under the cold glare of orbital searchlights",
            "a quiet observation deck aboard a fleet carrier, troop shuttles visible beyond the glass",
        ],
        "Navy" => &[
            "the cluttered bridge of a starship with glowing holo-displays",
            "a ship's corridor lit by cool blue strip lights",
            "a command bridge overlooking the illuminated curvature of a gas giant",
            "a compact sensor room crowded with tracking displays and communications equipment",
            "a warship's observation blister, with stars drifting beyond the armoured glass",
            "a naval academy simulator chamber with wraparound tactical projections",
            "a flight control deck overlooking a hangar full of small craft",
            "a shipboard mess hall during a quiet watch, officers gathered around metal tables",
            "a massive engineering compartment filled with reactor conduits and maintenance drones",
            "a fleet operations centre displaying dozens of vessels across a three-dimensional star map",
            "a captain's ready room with a model starship and a single large viewport",
            "a navigation station where plotted jump vectors hover above an illuminated table",
            "a brightly lit medical bay with diagnostic beds and emergency equipment",
            "a ship's airlock where pressure gauges glow beside a sealed inner hatch",
        ],
        "Scout" => &[
            "the cluttered bridge of a starship with glowing holo-displays",
            "a ship's corridor lit by cool blue strip lights",
            "a lonely landing pad overlooking a vast alien wilderness",
            "a remote survey station surrounded by antenna arrays and wind-battered equipment",
            "a mist-covered jungle with a compact scout ship visible through the trees",
            "a frozen alien plain beneath a sky dominated by a ringed gas giant",
            "a survey camp beside unusual rock formations and a field of portable instruments",
            "a weather station built into the cliffside of a desolate planet",
            "a rugged mountain ridge with a long-range scanner on a portable tripod",
            "a dark cave illuminated by helmet lamps and floating mapping drones",
            "a tiny frontier outpost surrounded by red dust and scattered supply containers",
            "the open ramp of a scout vessel overlooking a vast salt flat",
            "a deserted research outpost with flickering lights and a damaged communications mast",
            "a planetary orbital platform with survey craft docked against its weathered hull",
        ],
        "Merchant" => &[
            "a busy cargo bay with stacked containers and a loading drone",
            "a starport trading floor crowded with merchants",
            "a freighter's bridge with a commercial traffic display and a cracked pilot's chair",
            "a bustling customs hall filled with cargo inspectors and impatient hauliers",
            "a luxury liner's passenger concourse with polished floors and panoramic displays",
            "a cramped shipboard galley where crew members eat between cargo shifts",
            "a warehouse district filled with stacked shipping containers and grav forklifts",
            "a merchant exchange where traders negotiate under floating price displays",
            "a loading platform with cargo cranes moving containers between ships",
            "a shabby free-trader lounge with flickering lights and route maps on the wall",
            "a high-end auction house displaying rare artefacts beneath security barriers",
            "a commercial docking bay with fuel hoses, cargo loaders, and ships cycling through",
            "a freight office covered in shipping manifests and delivery schedules",
            "a passenger terminal overlooking a row of sleek interstellar liners",
        ],
        "Noble" => &[
            "a grand hall with tall arched windows and heraldic banners",
            "a private balcony above a glittering orbital city",
            "a formal court chamber with a raised throne and rows of waiting petitioners",
            "a luxurious garden beneath a transparent dome, with artificial birds among exotic plants",
            "a private dining room with crystal tableware and attendants in formal livery",
            "a traditional estate hall filled with antique weapons and portraits of ancestors",
            "a diplomatic reception overlooking the lights of a capital city",
            "a stately residence with polished marble floors and discreet security scanners",
            "a lavish orbital penthouse with a planet filling the panoramic windows",
            "a ceremonial parade ground where household guards stand beneath elaborate banners",
            "a secluded hunting lodge overlooking a rugged alien landscape",
            "a private art gallery filled with ancient sculptures and modern holographic installations",
            "a luxurious starport lounge reserved for dignitaries and senior officials",
            "a formal duelling chamber with polished floors and spectators behind safety barriers",
        ],
        "Scholar" => &[
            "a cluttered laboratory with holographic star charts",
            "a quiet library alcove lit by a warm lamp",
            "a university lecture theatre with a suspended holographic model of a planetary system",
            "a sterile research laboratory surrounded by sealed specimen cabinets",
            "an archaeological dig beneath a transparent weather canopy",
            "a crowded university office stacked with data slates and discarded coffee cups",
            "a high-security research facility with biometric doors and automated lab assistants",
            "a museum archive filled with ancient artefacts and climate-controlled display cases",
            "a field research camp overlooking mysterious ruins on a distant world",
            "a medical research ward with transparent diagnostic displays above each bed",
            "a vast digital library where millions of illuminated data entries float in virtual space",
            "an observatory control room with a giant telescope tracking distant stars",
            "a technical workshop filled with experimental instruments and half-built prototypes",
            "a planetary geology lab with rock samples, microscopes, and three-dimensional mineral scans",
        ],
        "Entertainer" => &[
            "backstage of a theatre, mirrors and soft spotlights",
            "a smoky lounge with coloured lights",
            "a packed nightclub with a holographic dance floor and pulsing light sculptures",
            "a luxurious concert hall with floating acoustic panels and a waiting audience",
            "a street performance stage in a crowded starport shopping arcade",
            "a private dressing room filled with costumes, cosmetics, and glowing vanity mirrors",
            "a television studio with hovering cameras and a wall of audience displays",
            "a lavish casino lounge with a small stage and discreet security staff",
            "an open-air amphitheatre beneath an alien night sky",
            "a recording studio surrounded by soundproof walls and holographic mixing consoles",
            "a crowded tavern where a travelling musician performs beside the bar",
            "a celebrity reception with holographic photographers and richly dressed guests",
            "a theatrical rehearsal room with marked floor panels and suspended lighting rigs",
            "a low-gravity performance arena where dancers and acrobats move above the stage",
        ],
        _ => &[
            "a noisy starport bar, warm amber light and smoke",
            "the edge of a landing pad at dusk, a battered freighter behind",
            "a crowded public concourse with travellers, luggage drones, and arrival announcements",
            "a dimly lit diner beside a busy interstellar highway",
            "a rain-soaked settlement street under flickering neon advertisements",
            "a dusty frontier market where traders sell goods beneath patched awnings",
            "a quiet residential corridor with numbered doors and a malfunctioning service robot",
            "a crowded orbital station promenade overlooking the curve of a planet",
            "a remote outpost surrounded by barren hills and a single illuminated landing pad",
            "a shabby hostel common room with mismatched furniture and a humming vending machine",
            "a converted cargo container serving as a temporary shop on a frontier world",
            "a small settlement square with a public information display and hovering delivery drones",
            "a dim maintenance bay filled with spare parts, tool carts, and a half-repaired vehicle",
            "a transport terminal where passengers queue beneath enormous departure boards",
            "a quiet rooftop terrace above a sprawling megacity, traffic glowing far below",
            "a wind-battered settlement on a frozen world, with lights shining through frosted windows",
        ],
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
