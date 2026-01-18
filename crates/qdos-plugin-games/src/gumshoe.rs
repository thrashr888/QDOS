//! GUMSHOE - Where in the World?
//!
//! Carmen Sandiego-style geography detective game. Chase criminals across
//! the globe, gather clues, and make arrests!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// GEOGRAPHY DATABASE
// =============================================================================

/// A city in the game world
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum City {
    Paris,
    London,
    Cairo,
    Tokyo,
    NewYork,
    Rome,
    Berlin,
    Sydney,
    RioDeJaneiro,
    Moscow,
    Beijing,
    Mumbai,
    MexicoCity,
    Nairobi,
    Dubai,
}

impl City {
    pub fn all() -> &'static [City] {
        &[
            City::Paris,
            City::London,
            City::Cairo,
            City::Tokyo,
            City::NewYork,
            City::Rome,
            City::Berlin,
            City::Sydney,
            City::RioDeJaneiro,
            City::Moscow,
            City::Beijing,
            City::Mumbai,
            City::MexicoCity,
            City::Nairobi,
            City::Dubai,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            City::Paris => "Paris",
            City::London => "London",
            City::Cairo => "Cairo",
            City::Tokyo => "Tokyo",
            City::NewYork => "New York",
            City::Rome => "Rome",
            City::Berlin => "Berlin",
            City::Sydney => "Sydney",
            City::RioDeJaneiro => "Rio de Janeiro",
            City::Moscow => "Moscow",
            City::Beijing => "Beijing",
            City::Mumbai => "Mumbai",
            City::MexicoCity => "Mexico City",
            City::Nairobi => "Nairobi",
            City::Dubai => "Dubai",
        }
    }

    pub fn country(&self) -> &'static str {
        match self {
            City::Paris => "France",
            City::London => "United Kingdom",
            City::Cairo => "Egypt",
            City::Tokyo => "Japan",
            City::NewYork => "USA",
            City::Rome => "Italy",
            City::Berlin => "Germany",
            City::Sydney => "Australia",
            City::RioDeJaneiro => "Brazil",
            City::Moscow => "Russia",
            City::Beijing => "China",
            City::Mumbai => "India",
            City::MexicoCity => "Mexico",
            City::Nairobi => "Kenya",
            City::Dubai => "UAE",
        }
    }

    pub fn continent(&self) -> &'static str {
        match self {
            City::Paris | City::London | City::Rome | City::Berlin | City::Moscow => "Europe",
            City::Cairo | City::Nairobi | City::Dubai => "Africa/Middle East",
            City::Tokyo | City::Beijing | City::Mumbai => "Asia",
            City::NewYork | City::MexicoCity => "North America",
            City::RioDeJaneiro => "South America",
            City::Sydney => "Oceania",
        }
    }

    pub fn landmark(&self) -> &'static str {
        match self {
            City::Paris => "Eiffel Tower",
            City::London => "Big Ben",
            City::Cairo => "Great Pyramids",
            City::Tokyo => "Tokyo Tower",
            City::NewYork => "Statue of Liberty",
            City::Rome => "Colosseum",
            City::Berlin => "Brandenburg Gate",
            City::Sydney => "Opera House",
            City::RioDeJaneiro => "Christ the Redeemer",
            City::Moscow => "Red Square",
            City::Beijing => "Great Wall",
            City::Mumbai => "Gateway of India",
            City::MexicoCity => "Aztec Pyramids",
            City::Nairobi => "National Park",
            City::Dubai => "Burj Khalifa",
        }
    }

    pub fn clue_hints(&self) -> &'static [&'static str] {
        match self {
            City::Paris => &[
                "mentioned croissants and baguettes",
                "wanted to see the Louvre",
                "spoke French fluently",
                "was heading to the city of lights",
            ],
            City::London => &[
                "asked about tea time",
                "wanted to see the Queen's palace",
                "mentioned double-decker buses",
                "was heading across the English Channel",
            ],
            City::Cairo => &[
                "interested in ancient pharaohs",
                "mentioned the Nile River",
                "wanted to see the Sphinx",
                "asked about desert tours",
            ],
            City::Tokyo => &[
                "fascinated by cherry blossoms",
                "wanted to try sushi",
                "mentioned bullet trains",
                "heading to the land of the rising sun",
            ],
            City::NewYork => &[
                "wanted to see Broadway shows",
                "mentioned the Big Apple",
                "interested in Central Park",
                "heading to the city that never sleeps",
            ],
            City::Rome => &[
                "wanted to throw coins in a fountain",
                "mentioned ancient gladiators",
                "interested in Vatican City",
                "heading to the Eternal City",
            ],
            City::Berlin => &[
                "interested in Cold War history",
                "mentioned the fallen wall",
                "wanted to see Museum Island",
                "heading to Germany's capital",
            ],
            City::Sydney => &[
                "wanted to see kangaroos",
                "mentioned the harbor bridge",
                "interested in the Great Barrier Reef",
                "heading down under",
            ],
            City::RioDeJaneiro => &[
                "excited about Carnival",
                "mentioned Copacabana beach",
                "wanted to see the giant statue",
                "heading to South America",
            ],
            City::Moscow => &[
                "interested in the Kremlin",
                "mentioned onion-domed churches",
                "wanted to see Red Square",
                "heading to Russia's capital",
            ],
            City::Beijing => &[
                "wanted to walk the ancient wall",
                "mentioned the Forbidden City",
                "interested in pandas",
                "heading to China",
            ],
            City::Mumbai => &[
                "interested in Bollywood",
                "mentioned spicy curry",
                "wanted to see colonial architecture",
                "heading to India's largest city",
            ],
            City::MexicoCity => &[
                "interested in Aztec history",
                "mentioned Day of the Dead",
                "wanted to try authentic tacos",
                "heading to Central America",
            ],
            City::Nairobi => &[
                "wanted to go on safari",
                "mentioned the Big Five animals",
                "interested in Maasai culture",
                "heading to East Africa",
            ],
            City::Dubai => &[
                "interested in the tallest building",
                "mentioned luxury shopping",
                "wanted to see the desert",
                "heading to the Persian Gulf",
            ],
        }
    }

    /// Flight time in hours to another city
    pub fn flight_time_to(&self, other: City) -> u32 {
        // Simplified - just use continent distance
        if *self == other {
            return 0;
        }
        let same_continent = self.continent() == other.continent();
        if same_continent {
            2
        } else {
            match (self.continent(), other.continent()) {
                ("Europe", "North America") | ("North America", "Europe") => 8,
                ("Europe", "Asia") | ("Asia", "Europe") => 10,
                ("North America", "Asia") | ("Asia", "North America") => 12,
                ("Europe", "Africa/Middle East") | ("Africa/Middle East", "Europe") => 4,
                ("Europe", "South America") | ("South America", "Europe") => 12,
                ("Europe", "Oceania") | ("Oceania", "Europe") => 20,
                _ => 10,
            }
        }
    }

    /// Flight cost to another city
    pub fn flight_cost_to(&self, other: City) -> u32 {
        self.flight_time_to(other) * 100
    }
}

// =============================================================================
// CRIMINALS
// =============================================================================

/// Hair color for suspect identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairColor {
    Black,
    Brown,
    Blonde,
    Red,
    Gray,
}

impl HairColor {
    pub fn name(&self) -> &'static str {
        match self {
            HairColor::Black => "Black",
            HairColor::Brown => "Brown",
            HairColor::Blonde => "Blonde",
            HairColor::Red => "Red",
            HairColor::Gray => "Gray",
        }
    }
}

/// A distinguishing feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Glasses,
    Scar,
    Tattoo,
    Jewelry,
    Hat,
}

impl Feature {
    pub fn name(&self) -> &'static str {
        match self {
            Feature::Glasses => "Wears glasses",
            Feature::Scar => "Has a scar",
            Feature::Tattoo => "Has tattoos",
            Feature::Jewelry => "Wears expensive jewelry",
            Feature::Hat => "Always wears a hat",
        }
    }
}

/// A hobby the suspect has
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hobby {
    Tennis,
    Painting,
    Music,
    Cooking,
    Reading,
}

impl Hobby {
    pub fn name(&self) -> &'static str {
        match self {
            Hobby::Tennis => "Plays tennis",
            Hobby::Painting => "Enjoys painting",
            Hobby::Music => "Loves music",
            Hobby::Cooking => "Gourmet cooking",
            Hobby::Reading => "Avid reader",
        }
    }
}

/// A criminal in the database
#[derive(Debug, Clone)]
pub struct Criminal {
    pub name: &'static str,
    pub gender: &'static str,
    pub hair: HairColor,
    pub feature: Feature,
    pub hobby: Hobby,
    pub difficulty: u32, // 1-3
}

const CRIMINALS: &[Criminal] = &[
    Criminal {
        name: "Carmen Sandiego",
        gender: "Female",
        hair: HairColor::Black,
        feature: Feature::Hat,
        hobby: Hobby::Reading,
        difficulty: 3,
    },
    Criminal {
        name: "Fast Eddie B.",
        gender: "Male",
        hair: HairColor::Brown,
        feature: Feature::Glasses,
        hobby: Hobby::Tennis,
        difficulty: 1,
    },
    Criminal {
        name: "Patty Larceny",
        gender: "Female",
        hair: HairColor::Blonde,
        feature: Feature::Jewelry,
        hobby: Hobby::Painting,
        difficulty: 2,
    },
    Criminal {
        name: "Nick Brunch",
        gender: "Male",
        hair: HairColor::Red,
        feature: Feature::Tattoo,
        hobby: Hobby::Cooking,
        difficulty: 1,
    },
    Criminal {
        name: "Contessa",
        gender: "Female",
        hair: HairColor::Gray,
        feature: Feature::Jewelry,
        hobby: Hobby::Music,
        difficulty: 2,
    },
    Criminal {
        name: "Dazzle Annie",
        gender: "Female",
        hair: HairColor::Red,
        feature: Feature::Glasses,
        hobby: Hobby::Painting,
        difficulty: 2,
    },
    Criminal {
        name: "Scar Graynolt",
        gender: "Male",
        hair: HairColor::Gray,
        feature: Feature::Scar,
        hobby: Hobby::Reading,
        difficulty: 3,
    },
    Criminal {
        name: "Top Grunge",
        gender: "Male",
        hair: HairColor::Black,
        feature: Feature::Tattoo,
        hobby: Hobby::Music,
        difficulty: 1,
    },
];

/// Stolen items
const STOLEN_ITEMS: &[&str] = &[
    "the Mona Lisa",
    "the Crown Jewels",
    "ancient Egyptian artifacts",
    "a rare diamond",
    "secret government documents",
    "a famous sculpture",
    "the original Declaration of Independence",
    "a priceless violin",
    "royal treasure",
    "a stolen masterpiece",
];

// =============================================================================
// GAME STATE
// =============================================================================

/// Current view in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GumshoeView {
    #[default]
    CaseIntro, // Starting a new case
    Map,         // Main world map view
    Investigate, // Investigating current location
    Witness,     // Talking to a witness
    Travel,      // Choosing destination
    Dossier,     // Viewing suspect info
    Arrest,      // Making an arrest
    CaseWon,     // Successfully caught criminal
    CaseLost,    // Ran out of time
    GameOver,    // No more cases or failed
}

/// A clue about the suspect's identity
#[derive(Debug, Clone)]
pub struct SuspectClue {
    pub clue_type: &'static str, // "Gender", "Hair", "Feature", "Hobby"
    pub value: String,
}

/// Current game state
#[derive(Debug, Clone)]
pub struct GumshoeState {
    // Game progress
    pub view: GumshoeView,
    pub game_over: bool,
    pub score: u32,
    pub rank_points: u32,
    pub cases_solved: u32,

    // Current case
    pub case_number: u32,
    pub criminal: Option<usize>, // Index into CRIMINALS
    pub stolen_item: String,
    pub trail: Vec<City>,      // Cities the criminal visits
    pub trail_position: usize, // Where criminal currently is
    pub current_city: City,
    pub time_remaining: u32, // Hours left
    pub money: u32,

    // Investigation
    pub investigated_airport: bool,
    pub investigated_hotel: bool,
    pub investigated_landmark: bool,
    pub last_clue: Option<String>,

    // Suspect identification
    pub suspect_clues: Vec<SuspectClue>,
    pub warrant_ready: bool,

    // UI state
    pub selected_destination: usize,
    pub selected_investigation: usize,
    pub selected_suspect: usize,

    // Events
    pending_events: Vec<GameEvent>,
}

impl Default for GumshoeState {
    fn default() -> Self {
        Self::new()
    }
}

impl GumshoeState {
    pub fn new() -> Self {
        let mut state = Self {
            view: GumshoeView::CaseIntro,
            game_over: false,
            score: 0,
            rank_points: 0,
            cases_solved: 0,
            case_number: 1,
            criminal: None,
            stolen_item: String::new(),
            trail: Vec::new(),
            trail_position: 0,
            current_city: City::Paris,
            time_remaining: 72, // 72 hours = 3 days
            money: 5000,
            investigated_airport: false,
            investigated_hotel: false,
            investigated_landmark: false,
            last_clue: None,
            suspect_clues: Vec::new(),
            warrant_ready: false,
            selected_destination: 0,
            selected_investigation: 0,
            selected_suspect: 0,
            pending_events: Vec::new(),
        };
        state.generate_case();
        state
    }

    /// Generate a new case
    pub fn generate_case(&mut self) {
        let mut rng = rand::thread_rng();

        // Pick a random criminal
        let criminal_idx = rng.gen_range(0..CRIMINALS.len());
        self.criminal = Some(criminal_idx);

        // Pick stolen item
        self.stolen_item = STOLEN_ITEMS[rng.gen_range(0..STOLEN_ITEMS.len())].to_string();

        // Generate trail of 3-5 cities
        let trail_length = rng.gen_range(3..=5);
        let mut available_cities: Vec<City> = City::all().to_vec();
        available_cities.shuffle(&mut rng);

        self.trail = available_cities.into_iter().take(trail_length).collect();
        self.trail_position = 0;

        // Start player at first city in trail
        self.current_city = self.trail[0];

        // Reset investigation state
        self.investigated_airport = false;
        self.investigated_hotel = false;
        self.investigated_landmark = false;
        self.last_clue = None;
        self.suspect_clues.clear();
        self.warrant_ready = false;

        // Set time based on trail length and difficulty
        let criminal = &CRIMINALS[criminal_idx];
        self.time_remaining = 48 + (trail_length as u32 * 12) - (criminal.difficulty * 8);

        self.view = GumshoeView::CaseIntro;
    }

    /// Get the current criminal
    pub fn get_criminal(&self) -> Option<&Criminal> {
        self.criminal.map(|idx| &CRIMINALS[idx])
    }

    /// Check if player is at the same city as criminal
    pub fn is_criminal_here(&self) -> bool {
        if self.trail_position >= self.trail.len() {
            return false;
        }
        self.current_city == self.trail[self.trail_position]
    }

    /// Investigate a location type
    pub fn investigate(&mut self, location_type: usize) {
        if self.time_remaining < 4 {
            self.last_clue = Some("Not enough time to investigate!".to_string());
            return;
        }

        self.time_remaining -= 4; // Each investigation takes 4 hours
        let mut rng = rand::thread_rng();

        // Determine if we get a destination clue or suspect clue
        let give_destination_clue = rng.gen_bool(0.6);

        match location_type {
            0 if !self.investigated_airport => {
                // Airport - usually gives destination clue
                self.investigated_airport = true;
                if give_destination_clue {
                    self.give_destination_clue();
                } else {
                    self.give_suspect_clue();
                }
            }
            1 if !self.investigated_hotel => {
                // Hotel - usually gives suspect clue
                self.investigated_hotel = true;
                if !give_destination_clue {
                    self.give_destination_clue();
                } else {
                    self.give_suspect_clue();
                }
            }
            2 if !self.investigated_landmark => {
                // Landmark - mixed
                self.investigated_landmark = true;
                if rng.gen_bool(0.5) {
                    self.give_destination_clue();
                } else {
                    self.give_suspect_clue();
                }
            }
            _ => {
                self.last_clue = Some("Already investigated here!".to_string());
            }
        }

        self.view = GumshoeView::Witness;
    }

    fn give_destination_clue(&mut self) {
        let mut rng = rand::thread_rng();

        // If criminal has moved on, point to next city
        let target_city = if self.trail_position + 1 < self.trail.len() {
            self.trail[self.trail_position + 1]
        } else {
            // Criminal is at final location
            self.trail[self.trail_position]
        };

        let hints = target_city.clue_hints();
        let hint = hints[rng.gen_range(0..hints.len())];
        self.last_clue = Some(format!("The suspect {}.", hint));
    }

    fn give_suspect_clue(&mut self) {
        let mut rng = rand::thread_rng();

        if let Some(criminal) = self.get_criminal() {
            // Pick a clue type we haven't given yet
            let given_types: Vec<&str> = self.suspect_clues.iter().map(|c| c.clue_type).collect();

            let available_clues: Vec<(&str, String)> = vec![
                ("Gender", criminal.gender.to_string()),
                ("Hair", criminal.hair.name().to_string()),
                ("Feature", criminal.feature.name().to_string()),
                ("Hobby", criminal.hobby.name().to_string()),
            ]
            .into_iter()
            .filter(|(t, _)| !given_types.contains(t))
            .collect();

            if let Some((clue_type, value)) = available_clues.into_iter().choose(&mut rng) {
                self.suspect_clues.push(SuspectClue {
                    clue_type,
                    value: value.clone(),
                });
                self.last_clue = Some(format!(
                    "The suspect's {}: {}.",
                    clue_type.to_lowercase(),
                    value
                ));

                // Check if warrant is ready (need 3+ clues)
                if self.suspect_clues.len() >= 3 {
                    self.warrant_ready = true;
                }
            } else {
                // All clues given, give destination instead
                self.give_destination_clue();
            }
        }
    }

    /// Travel to a new city
    pub fn travel_to(&mut self, city: City) {
        let flight_time = self.current_city.flight_time_to(city);
        let flight_cost = self.current_city.flight_cost_to(city);

        if self.money < flight_cost {
            self.last_clue = Some("Not enough money for this flight!".to_string());
            return;
        }

        if self.time_remaining < flight_time {
            self.last_clue = Some("Not enough time for this flight!".to_string());
            return;
        }

        self.money -= flight_cost;
        self.time_remaining -= flight_time;
        self.current_city = city;

        // Reset investigation state for new city
        self.investigated_airport = false;
        self.investigated_hotel = false;
        self.investigated_landmark = false;

        // Check if we caught up to criminal
        if self.is_criminal_here() && self.trail_position + 1 < self.trail.len() {
            // Criminal escapes to next city
            self.trail_position += 1;
        }

        // Check for time out
        if self.time_remaining == 0 {
            self.view = GumshoeView::CaseLost;
            return;
        }

        self.view = GumshoeView::Map;
    }

    /// Attempt to arrest
    pub fn attempt_arrest(&mut self, suspect_idx: usize) {
        if !self.warrant_ready {
            self.last_clue = Some("Need a warrant first! Gather more clues.".to_string());
            return;
        }

        if suspect_idx == self.criminal.unwrap_or(0) {
            // Correct arrest!
            let time_bonus = self.time_remaining * 10;
            let case_score = 1000 + time_bonus;
            self.score += case_score;
            self.rank_points += case_score;
            self.cases_solved += 1;
            self.view = GumshoeView::CaseWon;
            self.pending_events.push(GameEvent::Custom {
                key: "case_solved".to_string(),
                value: 1,
            });
        } else {
            // Wrong arrest - lose time
            self.time_remaining = self.time_remaining.saturating_sub(12);
            self.last_clue = Some("Wrong suspect! Lost 12 hours.".to_string());
            if self.time_remaining == 0 {
                self.view = GumshoeView::CaseLost;
            }
        }
    }

    /// Start next case after winning
    pub fn next_case(&mut self) {
        self.case_number += 1;
        self.money = 5000; // Reset budget
        self.generate_case();
    }

    /// Get list of destinations from current city
    pub fn get_destinations(&self) -> Vec<(City, u32, u32)> {
        City::all()
            .iter()
            .filter(|&&c| c != self.current_city)
            .map(|&c| {
                (
                    c,
                    self.current_city.flight_time_to(c),
                    self.current_city.flight_cost_to(c),
                )
            })
            .collect()
    }

    /// Get suspects that match current clues
    pub fn get_matching_suspects(&self) -> Vec<(usize, &'static Criminal, bool)> {
        CRIMINALS
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let matches = self.suspect_clues.iter().all(|clue| match clue.clue_type {
                    "Gender" => c.gender == clue.value,
                    "Hair" => c.hair.name() == clue.value,
                    "Feature" => c.feature.name() == clue.value,
                    "Hobby" => c.hobby.name() == clue.value,
                    _ => true,
                });
                (idx, c, matches)
            })
            .collect()
    }

    /// Get player rank title
    pub fn get_rank(&self) -> &'static str {
        match self.rank_points {
            0..=999 => "Rookie",
            1000..=2499 => "Detective",
            2500..=4999 => "Inspector",
            _ => "Chief",
        }
    }

    /// Get rank stars
    pub fn get_rank_stars(&self) -> &'static str {
        match self.rank_points {
            0..=999 => "*",
            1000..=2499 => "**",
            2500..=4999 => "***",
            _ => "****",
        }
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for GumshoeState {
    fn tick(&mut self) {
        // Time passes automatically during play
        // (handled by player actions instead)
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            GumshoeView::CaseIntro => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.view = GumshoeView::Map;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::Map => match key.code {
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.selected_investigation = 0;
                    self.view = GumshoeView::Investigate;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.selected_destination = 0;
                    self.view = GumshoeView::Travel;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.selected_suspect = 0;
                    self.view = GumshoeView::Dossier;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('a') | KeyCode::Char('A') if self.warrant_ready => {
                    self.selected_suspect = 0;
                    self.view = GumshoeView::Arrest;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::Investigate => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_investigation > 0 {
                        self.selected_investigation -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_investigation < 2 {
                        self.selected_investigation += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.investigate(self.selected_investigation);
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = GumshoeView::Map;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::Witness => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Esc => {
                    self.view = GumshoeView::Investigate;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::Travel => {
                let destinations = self.get_destinations();
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_destination > 0 {
                            self.selected_destination -= 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.selected_destination < destinations.len().saturating_sub(1) {
                            self.selected_destination += 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some((city, _, _)) = destinations.get(self.selected_destination) {
                            self.travel_to(*city);
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Esc => {
                        self.view = GumshoeView::Map;
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }

            GumshoeView::Dossier => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.view = GumshoeView::Map;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::Arrest => {
                let suspects = self.get_matching_suspects();
                let matching_count = suspects.iter().filter(|(_, _, m)| *m).count();

                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_suspect > 0 {
                            self.selected_suspect -= 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.selected_suspect < matching_count.saturating_sub(1) {
                            self.selected_suspect += 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        // Find the actual suspect index
                        let matching: Vec<_> = suspects.iter().filter(|(_, _, m)| *m).collect();
                        if let Some((idx, _, _)) = matching.get(self.selected_suspect) {
                            self.attempt_arrest(*idx);
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Esc => {
                        self.view = GumshoeView::Map;
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }

            GumshoeView::CaseWon => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.next_case();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.game_over = true;
                    KeyHandleResult::GameOver
                }
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::CaseLost => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Esc => {
                    self.game_over = true;
                    KeyHandleResult::GameOver
                }
                _ => KeyHandleResult::Handled,
            },

            GumshoeView::GameOver => KeyHandleResult::GameOver,
        }
    }

    fn get_score(&self) -> u32 {
        self.score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.cases_solved > 0
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.cases_solved)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
