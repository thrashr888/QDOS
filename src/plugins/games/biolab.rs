//! BIOLAB - Biology Learning Adventure
//!
//! An educational biology game with interactive labs, visual diagrams,
//! quizzes, and progress tracking.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_qmind::api::{chat::create_chat_provider, AIApiConfig};
use serde::Deserialize;

// =============================================================================
// ENUMS
// =============================================================================

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BiolabView {
    #[default]
    LabMenu,
    Microscope,
    DnaLab,
    Dissection,
    Anatomy,
    Quiz,
    QuizFeedback,
    QuizResults,
    Progress,
    Loading,
    Error,
}

/// Biology topics for quizzes and progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiologyTopic {
    CellStructure,
    PlantCells,
    Bacteria,
    BloodCells,
    Neurons,
    Muscles,
    DnaStructure,
    Genetics,
    FrogAnatomy,
    HumanNervous,
    HumanCirculatory,
    HumanRespiratory,
    HumanDigestive,
    HumanSkeletal,
}

impl BiologyTopic {
    pub fn name(&self) -> &'static str {
        match self {
            BiologyTopic::CellStructure => "Cell Structure",
            BiologyTopic::PlantCells => "Plant Cells",
            BiologyTopic::Bacteria => "Bacteria",
            BiologyTopic::BloodCells => "Blood Cells",
            BiologyTopic::Neurons => "Neurons",
            BiologyTopic::Muscles => "Muscle Tissue",
            BiologyTopic::DnaStructure => "DNA Structure",
            BiologyTopic::Genetics => "Genetics",
            BiologyTopic::FrogAnatomy => "Frog Anatomy",
            BiologyTopic::HumanNervous => "Nervous System",
            BiologyTopic::HumanCirculatory => "Circulatory System",
            BiologyTopic::HumanRespiratory => "Respiratory System",
            BiologyTopic::HumanDigestive => "Digestive System",
            BiologyTopic::HumanSkeletal => "Skeletal System",
        }
    }
}

/// Lab types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabType {
    Microscope,
    DnaLab,
    Dissection,
    Anatomy,
}

impl LabType {
    pub fn all() -> &'static [LabType] {
        &[
            LabType::Microscope,
            LabType::DnaLab,
            LabType::Dissection,
            LabType::Anatomy,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LabType::Microscope => "Microscope",
            LabType::DnaLab => "DNA Lab",
            LabType::Dissection => "Dissection",
            LabType::Anatomy => "Anatomy",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LabType::Microscope => "Explore cells under the microscope",
            LabType::DnaLab => "Study DNA structure and genetics",
            LabType::Dissection => "Virtual dissection of specimens",
            LabType::Anatomy => "Human body systems",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LabType::Microscope => "[*]",
            LabType::DnaLab => "~@~",
            LabType::Dissection => "(_)",
            LabType::Anatomy => "/|\\",
        }
    }
}

// =============================================================================
// MICROSCOPE LAB DATA
// =============================================================================

/// A cell organelle or structure
#[derive(Debug, Clone)]
pub struct Organelle {
    pub name: &'static str,
    pub description: &'static str,
    pub symbol: &'static str,
}

/// A microscope slide
#[derive(Debug, Clone)]
pub struct Slide {
    pub name: &'static str,
    pub topic: BiologyTopic,
    pub description: &'static str,
    pub organelles: &'static [Organelle],
    pub points: u32,
}

pub const SLIDES: &[Slide] = &[
    Slide {
        name: "Animal Cell",
        topic: BiologyTopic::CellStructure,
        description: "A typical eukaryotic animal cell with membrane-bound organelles.",
        organelles: &[
            Organelle {
                name: "Nucleus",
                description: "Control center containing DNA. Directs cell activities.",
                symbol: "@",
            },
            Organelle {
                name: "Mitochondria",
                description: "Powerhouse of the cell. Produces ATP through cellular respiration.",
                symbol: "O",
            },
            Organelle {
                name: "Endoplasmic Reticulum",
                description: "Network of membranes for protein and lipid synthesis.",
                symbol: "~",
            },
            Organelle {
                name: "Ribosomes",
                description: "Small structures that synthesize proteins from amino acids.",
                symbol: ".",
            },
            Organelle {
                name: "Golgi Apparatus",
                description: "Packages and ships proteins to their destinations.",
                symbol: "=",
            },
            Organelle {
                name: "Cell Membrane",
                description: "Selectively permeable barrier controlling what enters/exits.",
                symbol: "(",
            },
        ],
        points: 100,
    },
    Slide {
        name: "Plant Cell",
        topic: BiologyTopic::PlantCells,
        description: "A plant cell with cell wall, chloroplasts, and large central vacuole.",
        organelles: &[
            Organelle {
                name: "Cell Wall",
                description: "Rigid outer layer made of cellulose. Provides structure.",
                symbol: "#",
            },
            Organelle {
                name: "Chloroplast",
                description: "Site of photosynthesis. Contains chlorophyll pigment.",
                symbol: "*",
            },
            Organelle {
                name: "Central Vacuole",
                description: "Large water-filled sac for storage and maintaining turgor.",
                symbol: "V",
            },
            Organelle {
                name: "Nucleus",
                description: "Control center containing DNA.",
                symbol: "@",
            },
            Organelle {
                name: "Mitochondria",
                description: "Produces ATP for cellular energy.",
                symbol: "O",
            },
        ],
        points: 100,
    },
    Slide {
        name: "Bacteria",
        topic: BiologyTopic::Bacteria,
        description: "A prokaryotic bacterial cell - simpler than eukaryotes.",
        organelles: &[
            Organelle {
                name: "Cell Wall",
                description: "Protective outer layer made of peptidoglycan.",
                symbol: "#",
            },
            Organelle {
                name: "Flagellum",
                description: "Whip-like tail for movement.",
                symbol: "~",
            },
            Organelle {
                name: "Plasmid",
                description: "Small circular DNA separate from chromosome.",
                symbol: "o",
            },
            Organelle {
                name: "Ribosomes",
                description: "Protein synthesis machinery (smaller than eukaryotic).",
                symbol: ".",
            },
            Organelle {
                name: "Nucleoid",
                description: "Region containing the main chromosome (no membrane).",
                symbol: "@",
            },
        ],
        points: 75,
    },
    Slide {
        name: "Blood Cells",
        topic: BiologyTopic::BloodCells,
        description: "Human blood showing red cells, white cells, and platelets.",
        organelles: &[
            Organelle {
                name: "Red Blood Cell",
                description: "Carries oxygen using hemoglobin. No nucleus.",
                symbol: "O",
            },
            Organelle {
                name: "White Blood Cell",
                description: "Immune defense. Fights infections and disease.",
                symbol: "@",
            },
            Organelle {
                name: "Platelet",
                description: "Cell fragments for blood clotting.",
                symbol: ".",
            },
            Organelle {
                name: "Plasma",
                description: "Liquid portion carrying nutrients and waste.",
                symbol: "-",
            },
        ],
        points: 75,
    },
    Slide {
        name: "Neuron",
        topic: BiologyTopic::Neurons,
        description: "A nerve cell specialized for transmitting electrical signals.",
        organelles: &[
            Organelle {
                name: "Cell Body (Soma)",
                description: "Contains the nucleus and most organelles.",
                symbol: "@",
            },
            Organelle {
                name: "Dendrites",
                description: "Branch-like extensions receiving signals from other neurons.",
                symbol: "<",
            },
            Organelle {
                name: "Axon",
                description: "Long fiber transmitting signals away from cell body.",
                symbol: "-",
            },
            Organelle {
                name: "Myelin Sheath",
                description: "Insulating layer speeding up signal transmission.",
                symbol: "=",
            },
            Organelle {
                name: "Synapse",
                description: "Junction where signals pass to the next neuron.",
                symbol: ">",
            },
        ],
        points: 100,
    },
    Slide {
        name: "Muscle Fiber",
        topic: BiologyTopic::Muscles,
        description: "Skeletal muscle tissue showing striated fibers.",
        organelles: &[
            Organelle {
                name: "Muscle Fiber",
                description: "Long cylindrical cell containing many nuclei.",
                symbol: "|",
            },
            Organelle {
                name: "Sarcomere",
                description: "Basic contractile unit. Shortens during contraction.",
                symbol: "#",
            },
            Organelle {
                name: "Striations",
                description: "Dark and light bands from organized proteins.",
                symbol: "=",
            },
            Organelle {
                name: "Nuclei",
                description: "Multiple nuclei along the fiber's edge.",
                symbol: "@",
            },
        ],
        points: 75,
    },
];

// =============================================================================
// DNA LAB DATA
// =============================================================================

/// DNA base pairs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaBase {
    Adenine,
    Thymine,
    Guanine,
    Cytosine,
}

impl DnaBase {
    pub fn char(&self) -> char {
        match self {
            DnaBase::Adenine => 'A',
            DnaBase::Thymine => 'T',
            DnaBase::Guanine => 'G',
            DnaBase::Cytosine => 'C',
        }
    }

    pub fn complement(&self) -> DnaBase {
        match self {
            DnaBase::Adenine => DnaBase::Thymine,
            DnaBase::Thymine => DnaBase::Adenine,
            DnaBase::Guanine => DnaBase::Cytosine,
            DnaBase::Cytosine => DnaBase::Guanine,
        }
    }

    pub fn rna_complement(&self) -> char {
        match self {
            DnaBase::Adenine => 'U',
            DnaBase::Thymine => 'A',
            DnaBase::Guanine => 'C',
            DnaBase::Cytosine => 'G',
        }
    }

    pub fn from_char(c: char) -> Option<DnaBase> {
        match c.to_ascii_uppercase() {
            'A' => Some(DnaBase::Adenine),
            'T' => Some(DnaBase::Thymine),
            'G' => Some(DnaBase::Guanine),
            'C' => Some(DnaBase::Cytosine),
            _ => None,
        }
    }
}

/// DNA Lab sub-views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DnaLabMode {
    #[default]
    Helix,
    Build,
    Transcription,
}

// =============================================================================
// DISSECTION LAB DATA
// =============================================================================

/// A body part in a specimen
#[derive(Debug, Clone)]
pub struct BodyPart {
    pub name: &'static str,
    pub description: &'static str,
    pub function: &'static str,
}

/// A dissection specimen
#[derive(Debug, Clone)]
pub struct Specimen {
    pub name: &'static str,
    pub topic: BiologyTopic,
    pub description: &'static str,
    pub parts: &'static [BodyPart],
    pub points: u32,
}

pub const SPECIMENS: &[Specimen] = &[
    Specimen {
        name: "Frog",
        topic: BiologyTopic::FrogAnatomy,
        description: "Classic dissection specimen showing vertebrate organ systems.",
        parts: &[
            BodyPart {
                name: "Heart",
                description: "Three-chambered heart (2 atria, 1 ventricle)",
                function: "Pumps blood through body",
            },
            BodyPart {
                name: "Lungs",
                description: "Simple sac-like lungs",
                function: "Gas exchange (also uses skin)",
            },
            BodyPart {
                name: "Liver",
                description: "Large, dark organ",
                function: "Processes nutrients, detoxification",
            },
            BodyPart {
                name: "Stomach",
                description: "J-shaped digestive organ",
                function: "Breaks down food with acid",
            },
            BodyPart {
                name: "Intestines",
                description: "Coiled tubes",
                function: "Nutrient absorption",
            },
        ],
        points: 100,
    },
    Specimen {
        name: "Earthworm",
        topic: BiologyTopic::FrogAnatomy, // Using closest topic
        description: "Segmented worm showing simple organ systems.",
        parts: &[
            BodyPart {
                name: "Segments",
                description: "Repeating body sections",
                function: "Allow flexibility and movement",
            },
            BodyPart {
                name: "Aortic Arches",
                description: "5 pairs of 'hearts'",
                function: "Pump blood through body",
            },
            BodyPart {
                name: "Crop",
                description: "Storage chamber",
                function: "Stores food before digestion",
            },
            BodyPart {
                name: "Gizzard",
                description: "Muscular grinding organ",
                function: "Mechanically breaks down food",
            },
        ],
        points: 75,
    },
    Specimen {
        name: "Fish",
        topic: BiologyTopic::FrogAnatomy,
        description: "Bony fish showing aquatic adaptations.",
        parts: &[
            BodyPart {
                name: "Gills",
                description: "Feathery red structures",
                function: "Extract oxygen from water",
            },
            BodyPart {
                name: "Swim Bladder",
                description: "Gas-filled sac",
                function: "Controls buoyancy",
            },
            BodyPart {
                name: "Heart",
                description: "Two-chambered heart",
                function: "Pumps blood in single loop",
            },
            BodyPart {
                name: "Lateral Line",
                description: "Row of sensory organs",
                function: "Detects water pressure changes",
            },
        ],
        points: 75,
    },
    Specimen {
        name: "Flower",
        topic: BiologyTopic::PlantCells,
        description: "Flowering plant reproductive structure.",
        parts: &[
            BodyPart {
                name: "Petals",
                description: "Colorful outer parts",
                function: "Attract pollinators",
            },
            BodyPart {
                name: "Stamen",
                description: "Male reproductive organ",
                function: "Produces pollen (sperm)",
            },
            BodyPart {
                name: "Pistil",
                description: "Female reproductive organ",
                function: "Contains ovules (eggs)",
            },
            BodyPart {
                name: "Ovary",
                description: "Base of pistil",
                function: "Develops into fruit/seeds",
            },
        ],
        points: 75,
    },
    Specimen {
        name: "Cow Eye",
        topic: BiologyTopic::Neurons, // Sensory organ
        description: "Mammalian eye structure.",
        parts: &[
            BodyPart {
                name: "Cornea",
                description: "Clear outer layer",
                function: "Focuses light, protects eye",
            },
            BodyPart {
                name: "Lens",
                description: "Flexible clear disc",
                function: "Fine-tunes focus on retina",
            },
            BodyPart {
                name: "Retina",
                description: "Layer at back of eye",
                function: "Contains light-sensitive cells",
            },
            BodyPart {
                name: "Optic Nerve",
                description: "Bundle of nerve fibers",
                function: "Carries signals to brain",
            },
        ],
        points: 100,
    },
];

// =============================================================================
// ANATOMY LAB DATA
// =============================================================================

/// A component of a body system
#[derive(Debug, Clone)]
pub struct SystemPart {
    pub name: &'static str,
    pub description: &'static str,
}

/// A human body system
#[derive(Debug, Clone)]
pub struct BodySystem {
    pub name: &'static str,
    pub topic: BiologyTopic,
    pub description: &'static str,
    pub parts: &'static [SystemPart],
    pub points: u32,
}

pub const BODY_SYSTEMS: &[BodySystem] = &[
    BodySystem {
        name: "Nervous System",
        topic: BiologyTopic::HumanNervous,
        description: "Controls and coordinates body activities through electrical signals.",
        parts: &[
            SystemPart {
                name: "Brain",
                description: "Control center for thought, memory, emotion, movement",
            },
            SystemPart {
                name: "Spinal Cord",
                description: "Highway connecting brain to body nerves",
            },
            SystemPart {
                name: "Peripheral Nerves",
                description: "Network carrying signals to muscles and organs",
            },
            SystemPart {
                name: "Sensory Receptors",
                description: "Detect stimuli (touch, pain, temperature)",
            },
        ],
        points: 100,
    },
    BodySystem {
        name: "Circulatory System",
        topic: BiologyTopic::HumanCirculatory,
        description: "Transports blood, nutrients, and oxygen throughout the body.",
        parts: &[
            SystemPart {
                name: "Heart",
                description: "Four-chambered pump driving blood flow",
            },
            SystemPart {
                name: "Arteries",
                description: "Carry oxygenated blood away from heart",
            },
            SystemPart {
                name: "Veins",
                description: "Return deoxygenated blood to heart",
            },
            SystemPart {
                name: "Capillaries",
                description: "Tiny vessels for nutrient/gas exchange",
            },
        ],
        points: 100,
    },
    BodySystem {
        name: "Respiratory System",
        topic: BiologyTopic::HumanRespiratory,
        description: "Brings oxygen into body and removes carbon dioxide.",
        parts: &[
            SystemPart {
                name: "Lungs",
                description: "Spongy organs where gas exchange occurs",
            },
            SystemPart {
                name: "Trachea",
                description: "Windpipe carrying air to lungs",
            },
            SystemPart {
                name: "Bronchi",
                description: "Branching tubes in lungs",
            },
            SystemPart {
                name: "Diaphragm",
                description: "Muscle controlling breathing",
            },
        ],
        points: 75,
    },
    BodySystem {
        name: "Digestive System",
        topic: BiologyTopic::HumanDigestive,
        description: "Breaks down food and absorbs nutrients.",
        parts: &[
            SystemPart {
                name: "Mouth",
                description: "Mechanical and chemical digestion begins",
            },
            SystemPart {
                name: "Esophagus",
                description: "Tube connecting mouth to stomach",
            },
            SystemPart {
                name: "Stomach",
                description: "Churns food with acid and enzymes",
            },
            SystemPart {
                name: "Small Intestine",
                description: "Main site of nutrient absorption",
            },
            SystemPart {
                name: "Large Intestine",
                description: "Absorbs water, forms waste",
            },
        ],
        points: 100,
    },
    BodySystem {
        name: "Skeletal System",
        topic: BiologyTopic::HumanSkeletal,
        description: "Provides structure, protection, and enables movement.",
        parts: &[
            SystemPart {
                name: "Skull",
                description: "Protects the brain",
            },
            SystemPart {
                name: "Spine",
                description: "Supports body, protects spinal cord",
            },
            SystemPart {
                name: "Ribs",
                description: "Protect heart and lungs",
            },
            SystemPart {
                name: "Limb Bones",
                description: "Enable movement and manipulation",
            },
        ],
        points: 75,
    },
];

// =============================================================================
// QUIZ DATA
// =============================================================================

/// A quiz question
#[derive(Debug, Clone)]
pub struct BiologyQuestion {
    pub question: String,
    pub options: [String; 4],
    pub correct_index: usize,
    pub explanation: String,
    pub topic: BiologyTopic,
}

/// Fallback questions when API is unavailable
pub fn fallback_questions(topic: BiologyTopic) -> Vec<BiologyQuestion> {
    match topic {
        BiologyTopic::CellStructure => vec![
            BiologyQuestion {
                question: "What organelle is called the 'powerhouse of the cell'?".to_string(),
                options: [
                    "Nucleus".to_string(),
                    "Mitochondria".to_string(),
                    "Ribosome".to_string(),
                    "Golgi Apparatus".to_string(),
                ],
                correct_index: 1,
                explanation: "Mitochondria produce ATP, the cell's energy currency.".to_string(),
                topic: BiologyTopic::CellStructure,
            },
            BiologyQuestion {
                question: "Which structure contains the cell's genetic material?".to_string(),
                options: [
                    "Vacuole".to_string(),
                    "Cell membrane".to_string(),
                    "Nucleus".to_string(),
                    "Cytoplasm".to_string(),
                ],
                correct_index: 2,
                explanation: "The nucleus houses DNA and controls cell activities.".to_string(),
                topic: BiologyTopic::CellStructure,
            },
            BiologyQuestion {
                question: "What is the function of ribosomes?".to_string(),
                options: [
                    "Energy production".to_string(),
                    "Protein synthesis".to_string(),
                    "Storage".to_string(),
                    "Cell division".to_string(),
                ],
                correct_index: 1,
                explanation: "Ribosomes read mRNA and assemble amino acids into proteins."
                    .to_string(),
                topic: BiologyTopic::CellStructure,
            },
            BiologyQuestion {
                question: "The cell membrane is described as 'selectively permeable'. What does this mean?".to_string(),
                options: [
                    "Nothing can pass through".to_string(),
                    "Everything can pass through".to_string(),
                    "Only certain substances can pass".to_string(),
                    "Only water can pass".to_string(),
                ],
                correct_index: 2,
                explanation: "The membrane controls what enters and exits the cell.".to_string(),
                topic: BiologyTopic::CellStructure,
            },
            BiologyQuestion {
                question: "Which organelle modifies, packages, and ships proteins?".to_string(),
                options: [
                    "Ribosome".to_string(),
                    "Endoplasmic Reticulum".to_string(),
                    "Golgi Apparatus".to_string(),
                    "Lysosome".to_string(),
                ],
                correct_index: 2,
                explanation: "The Golgi apparatus is like the cell's post office.".to_string(),
                topic: BiologyTopic::CellStructure,
            },
        ],
        BiologyTopic::PlantCells => vec![
            BiologyQuestion {
                question: "What structure do plant cells have that animal cells lack?".to_string(),
                options: [
                    "Nucleus".to_string(),
                    "Cell wall".to_string(),
                    "Mitochondria".to_string(),
                    "Ribosomes".to_string(),
                ],
                correct_index: 1,
                explanation: "Plant cells have a rigid cell wall made of cellulose.".to_string(),
                topic: BiologyTopic::PlantCells,
            },
            BiologyQuestion {
                question: "Where does photosynthesis occur in plant cells?".to_string(),
                options: [
                    "Mitochondria".to_string(),
                    "Nucleus".to_string(),
                    "Chloroplast".to_string(),
                    "Vacuole".to_string(),
                ],
                correct_index: 2,
                explanation: "Chloroplasts contain chlorophyll and convert light to energy."
                    .to_string(),
                topic: BiologyTopic::PlantCells,
            },
            BiologyQuestion {
                question: "What is the large water-filled sac in plant cells called?".to_string(),
                options: [
                    "Lysosome".to_string(),
                    "Central vacuole".to_string(),
                    "Golgi body".to_string(),
                    "Vesicle".to_string(),
                ],
                correct_index: 1,
                explanation: "The central vacuole stores water and maintains turgor pressure."
                    .to_string(),
                topic: BiologyTopic::PlantCells,
            },
            BiologyQuestion {
                question: "What pigment makes plants green?".to_string(),
                options: [
                    "Melanin".to_string(),
                    "Hemoglobin".to_string(),
                    "Chlorophyll".to_string(),
                    "Carotene".to_string(),
                ],
                correct_index: 2,
                explanation: "Chlorophyll absorbs red and blue light, reflecting green."
                    .to_string(),
                topic: BiologyTopic::PlantCells,
            },
            BiologyQuestion {
                question: "What is the main component of plant cell walls?".to_string(),
                options: [
                    "Protein".to_string(),
                    "Lipid".to_string(),
                    "Cellulose".to_string(),
                    "Starch".to_string(),
                ],
                correct_index: 2,
                explanation: "Cellulose is a strong carbohydrate that provides structure."
                    .to_string(),
                topic: BiologyTopic::PlantCells,
            },
        ],
        BiologyTopic::DnaStructure | BiologyTopic::Genetics => vec![
            BiologyQuestion {
                question: "What shape is DNA?".to_string(),
                options: [
                    "Single helix".to_string(),
                    "Double helix".to_string(),
                    "Triple helix".to_string(),
                    "Flat sheet".to_string(),
                ],
                correct_index: 1,
                explanation: "DNA is a twisted ladder shape called a double helix.".to_string(),
                topic: BiologyTopic::DnaStructure,
            },
            BiologyQuestion {
                question: "Adenine (A) pairs with which base in DNA?".to_string(),
                options: [
                    "Cytosine".to_string(),
                    "Guanine".to_string(),
                    "Thymine".to_string(),
                    "Uracil".to_string(),
                ],
                correct_index: 2,
                explanation: "A-T and G-C are the complementary base pairs in DNA.".to_string(),
                topic: BiologyTopic::DnaStructure,
            },
            BiologyQuestion {
                question: "What is a codon?".to_string(),
                options: [
                    "A single nucleotide".to_string(),
                    "A sequence of 3 nucleotides".to_string(),
                    "A protein".to_string(),
                    "A chromosome".to_string(),
                ],
                correct_index: 1,
                explanation: "Codons are triplets that code for specific amino acids.".to_string(),
                topic: BiologyTopic::Genetics,
            },
            BiologyQuestion {
                question: "What process copies DNA before cell division?".to_string(),
                options: [
                    "Transcription".to_string(),
                    "Translation".to_string(),
                    "Replication".to_string(),
                    "Mutation".to_string(),
                ],
                correct_index: 2,
                explanation: "DNA replication creates an exact copy of the genetic material."
                    .to_string(),
                topic: BiologyTopic::DnaStructure,
            },
            BiologyQuestion {
                question: "In mRNA, what base replaces thymine?".to_string(),
                options: [
                    "Adenine".to_string(),
                    "Guanine".to_string(),
                    "Cytosine".to_string(),
                    "Uracil".to_string(),
                ],
                correct_index: 3,
                explanation: "RNA uses uracil (U) instead of thymine (T).".to_string(),
                topic: BiologyTopic::DnaStructure,
            },
        ],
        BiologyTopic::HumanCirculatory => vec![
            BiologyQuestion {
                question: "How many chambers does the human heart have?".to_string(),
                options: [
                    "Two".to_string(),
                    "Three".to_string(),
                    "Four".to_string(),
                    "Five".to_string(),
                ],
                correct_index: 2,
                explanation: "The heart has 2 atria and 2 ventricles.".to_string(),
                topic: BiologyTopic::HumanCirculatory,
            },
            BiologyQuestion {
                question: "Which blood vessels carry blood AWAY from the heart?".to_string(),
                options: [
                    "Veins".to_string(),
                    "Arteries".to_string(),
                    "Capillaries".to_string(),
                    "Lymph vessels".to_string(),
                ],
                correct_index: 1,
                explanation: "Arteries carry blood away; veins return it to the heart.".to_string(),
                topic: BiologyTopic::HumanCirculatory,
            },
            BiologyQuestion {
                question: "What protein in red blood cells carries oxygen?".to_string(),
                options: [
                    "Insulin".to_string(),
                    "Keratin".to_string(),
                    "Hemoglobin".to_string(),
                    "Collagen".to_string(),
                ],
                correct_index: 2,
                explanation: "Hemoglobin binds oxygen and gives blood its red color.".to_string(),
                topic: BiologyTopic::HumanCirculatory,
            },
            BiologyQuestion {
                question: "Where does gas exchange occur in the circulatory system?".to_string(),
                options: [
                    "Heart".to_string(),
                    "Arteries".to_string(),
                    "Veins".to_string(),
                    "Capillaries".to_string(),
                ],
                correct_index: 3,
                explanation: "Capillaries have thin walls allowing O2/CO2 exchange.".to_string(),
                topic: BiologyTopic::HumanCirculatory,
            },
            BiologyQuestion {
                question: "What are the smallest blood vessels called?".to_string(),
                options: [
                    "Arteries".to_string(),
                    "Veins".to_string(),
                    "Capillaries".to_string(),
                    "Aorta".to_string(),
                ],
                correct_index: 2,
                explanation: "Capillaries are microscopic vessels one cell thick.".to_string(),
                topic: BiologyTopic::HumanCirculatory,
            },
        ],
        _ => vec![
            BiologyQuestion {
                question: format!("What is the main focus of {} studies?", topic.name()),
                options: [
                    "Cell structure".to_string(),
                    "Genetics".to_string(),
                    "Anatomy".to_string(),
                    topic.name().to_string(),
                ],
                correct_index: 3,
                explanation: format!("This topic covers {} concepts.", topic.name()),
                topic,
            },
            BiologyQuestion {
                question: "Which scientist is known as the father of genetics?".to_string(),
                options: [
                    "Darwin".to_string(),
                    "Mendel".to_string(),
                    "Watson".to_string(),
                    "Pasteur".to_string(),
                ],
                correct_index: 1,
                explanation: "Gregor Mendel discovered inheritance patterns using pea plants."
                    .to_string(),
                topic,
            },
            BiologyQuestion {
                question: "What is the basic unit of life?".to_string(),
                options: [
                    "Atom".to_string(),
                    "Molecule".to_string(),
                    "Cell".to_string(),
                    "Organ".to_string(),
                ],
                correct_index: 2,
                explanation: "All living things are made of one or more cells.".to_string(),
                topic,
            },
            BiologyQuestion {
                question: "What molecule stores genetic information?".to_string(),
                options: [
                    "Protein".to_string(),
                    "Lipid".to_string(),
                    "DNA".to_string(),
                    "Carbohydrate".to_string(),
                ],
                correct_index: 2,
                explanation: "DNA (deoxyribonucleic acid) contains the genetic code.".to_string(),
                topic,
            },
            BiologyQuestion {
                question: "What is homeostasis?".to_string(),
                options: [
                    "Cell division".to_string(),
                    "Maintaining stable internal conditions".to_string(),
                    "Energy production".to_string(),
                    "Reproduction".to_string(),
                ],
                correct_index: 1,
                explanation: "Homeostasis keeps body conditions like temperature stable."
                    .to_string(),
                topic,
            },
        ],
    }
}

// =============================================================================
// STATE
// =============================================================================

/// Main BIOLAB game state
pub struct BiolabState {
    // View state
    pub view: BiolabView,
    pub previous_view: BiolabView,

    // Lab navigation
    pub selected_lab: usize,
    pub selected_slide: usize,
    pub selected_specimen: usize,
    pub selected_system: usize,
    pub selected_organelle: usize,
    pub selected_part: usize,

    // DNA Lab state
    pub dna_mode: DnaLabMode,
    pub dna_sequence: Vec<DnaBase>,
    pub dna_cursor: usize,

    // Quiz state
    pub quiz_topic: BiologyTopic,
    pub quiz_questions: Vec<BiologyQuestion>,
    pub current_question: usize,
    pub selected_answer: usize,
    pub quiz_correct: u32,
    pub show_explanation: bool,
    pub pending_generation: bool,

    // Progress tracking
    pub slides_viewed: Vec<bool>,
    pub specimens_viewed: Vec<bool>,
    pub systems_viewed: Vec<bool>,
    pub topics_quizzed: Vec<BiologyTopic>,
    pub topics_mastered: Vec<BiologyTopic>,
    pub total_score: u32,
    pub quizzes_taken: u32,
    pub perfect_quizzes: u32,

    // System
    pub error_message: Option<String>,
    pub api_available: bool,
    pub game_over: bool,
    pending_events: Vec<GameEvent>,
}

impl Default for BiolabState {
    fn default() -> Self {
        Self::new()
    }
}

impl BiolabState {
    pub fn new() -> Self {
        let api_config = AIApiConfig::from_env();
        Self {
            view: BiolabView::LabMenu,
            previous_view: BiolabView::LabMenu,
            selected_lab: 0,
            selected_slide: 0,
            selected_specimen: 0,
            selected_system: 0,
            selected_organelle: 0,
            selected_part: 0,
            dna_mode: DnaLabMode::Helix,
            dna_sequence: vec![
                DnaBase::Adenine,
                DnaBase::Thymine,
                DnaBase::Guanine,
                DnaBase::Cytosine,
                DnaBase::Cytosine,
                DnaBase::Adenine,
                DnaBase::Thymine,
            ],
            dna_cursor: 0,
            quiz_topic: BiologyTopic::CellStructure,
            quiz_questions: Vec::new(),
            current_question: 0,
            selected_answer: 0,
            quiz_correct: 0,
            show_explanation: false,
            pending_generation: false,
            slides_viewed: vec![false; SLIDES.len()],
            specimens_viewed: vec![false; SPECIMENS.len()],
            systems_viewed: vec![false; BODY_SYSTEMS.len()],
            topics_quizzed: Vec::new(),
            topics_mastered: Vec::new(),
            total_score: 0,
            quizzes_taken: 0,
            perfect_quizzes: 0,
            error_message: None,
            api_available: api_config.is_configured(),
            game_over: false,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    pub fn enter_lab(&mut self, lab: LabType) {
        self.previous_view = self.view;
        match lab {
            LabType::Microscope => {
                self.view = BiolabView::Microscope;
                self.selected_organelle = 0;
                self.mark_slide_viewed();
            }
            LabType::DnaLab => {
                self.view = BiolabView::DnaLab;
                self.dna_mode = DnaLabMode::Helix;
            }
            LabType::Dissection => {
                self.view = BiolabView::Dissection;
                self.selected_part = 0;
                self.mark_specimen_viewed();
            }
            LabType::Anatomy => {
                self.view = BiolabView::Anatomy;
                self.selected_part = 0;
                self.mark_system_viewed();
            }
        }
    }

    pub fn back_to_menu(&mut self) {
        self.view = BiolabView::LabMenu;
    }

    // =========================================================================
    // Progress Tracking
    // =========================================================================

    fn mark_slide_viewed(&mut self) {
        if self.selected_slide < self.slides_viewed.len()
            && !self.slides_viewed[self.selected_slide]
        {
            self.slides_viewed[self.selected_slide] = true;
            self.total_score += 10;
            self.pending_events.push(GameEvent::ScoreChanged {
                old: self.total_score - 10,
                new: self.total_score,
            });
        }
    }

    fn mark_specimen_viewed(&mut self) {
        if self.selected_specimen < self.specimens_viewed.len()
            && !self.specimens_viewed[self.selected_specimen]
        {
            self.specimens_viewed[self.selected_specimen] = true;
            self.total_score += 10;
        }
    }

    fn mark_system_viewed(&mut self) {
        if self.selected_system < self.systems_viewed.len()
            && !self.systems_viewed[self.selected_system]
        {
            self.systems_viewed[self.selected_system] = true;
            self.total_score += 10;
        }
    }

    pub fn all_slides_viewed(&self) -> bool {
        self.slides_viewed.iter().all(|&v| v)
    }

    pub fn all_specimens_viewed(&self) -> bool {
        self.specimens_viewed.iter().all(|&v| v)
    }

    pub fn all_systems_viewed(&self) -> bool {
        self.systems_viewed.iter().all(|&v| v)
    }

    // =========================================================================
    // Quiz System
    // =========================================================================

    pub fn start_quiz(&mut self, topic: BiologyTopic) {
        self.quiz_topic = topic;
        self.quiz_questions.clear();
        self.current_question = 0;
        self.selected_answer = 0;
        self.quiz_correct = 0;
        self.show_explanation = false;
        self.pending_generation = true;
        self.previous_view = self.view;
        self.view = BiolabView::Loading;
    }

    pub fn generate_quiz(&mut self) {
        self.pending_generation = false;

        // Try AI generation first
        if self.api_available {
            if let Some(questions) = self.generate_ai_questions() {
                self.quiz_questions = questions;
                self.view = BiolabView::Quiz;
                return;
            }
        }

        // Fall back to static questions
        self.quiz_questions = fallback_questions(self.quiz_topic);
        self.view = BiolabView::Quiz;
    }

    fn generate_ai_questions(&mut self) -> Option<Vec<BiologyQuestion>> {
        let config = AIApiConfig::from_env();
        let provider = match create_chat_provider(config) {
            Ok(p) => p,
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                return None;
            }
        };

        let system_prompt = "You are a biology quiz generator for high school students. Generate engaging, educational multiple choice questions.";

        let user_prompt = format!(
            r#"Generate 5 multiple choice biology questions about "{}".
Return valid JSON array with this exact format:
[
  {{
    "question": "Question text here?",
    "options": ["Option A", "Option B", "Option C", "Option D"],
    "correct_index": 0,
    "explanation": "Brief explanation of correct answer"
  }}
]
Make questions age-appropriate for 15-year-olds. Include a mix of difficulty levels."#,
            self.quiz_topic.name()
        );

        match provider.prompt(Some(system_prompt), &user_prompt) {
            Ok(response) => {
                if let Ok(questions) = parse_quiz_response(&response.content, self.quiz_topic) {
                    Some(questions)
                } else {
                    self.error_message = Some("Failed to parse quiz response".to_string());
                    None
                }
            }
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                None
            }
        }
    }

    pub fn answer_question(&mut self, answer: usize) {
        self.selected_answer = answer;

        if let Some(question) = self.quiz_questions.get(self.current_question) {
            if answer == question.correct_index {
                self.quiz_correct += 1;
                self.total_score += 20;
            }
        }

        self.show_explanation = true;
        self.view = BiolabView::QuizFeedback;
    }

    pub fn next_question(&mut self) {
        self.show_explanation = false;
        self.current_question += 1;
        self.selected_answer = 0;

        if self.current_question >= self.quiz_questions.len() {
            self.finish_quiz();
        } else {
            self.view = BiolabView::Quiz;
        }
    }

    fn finish_quiz(&mut self) {
        self.quizzes_taken += 1;

        let percent = if !self.quiz_questions.is_empty() {
            (self.quiz_correct as f32 / self.quiz_questions.len() as f32 * 100.0) as u32
        } else {
            0
        };

        // Perfect quiz bonus
        if self.quiz_correct == self.quiz_questions.len() as u32 {
            self.perfect_quizzes += 1;
            self.total_score += 50;
        }

        // Topic mastery (>= 80%)
        if percent >= 80 && !self.topics_mastered.contains(&self.quiz_topic) {
            self.topics_mastered.push(self.quiz_topic);
            self.total_score += 100;
        }

        // Track quizzed topics
        if !self.topics_quizzed.contains(&self.quiz_topic) {
            self.topics_quizzed.push(self.quiz_topic);
        }

        self.view = BiolabView::QuizResults;
    }

    pub fn current_slide(&self) -> &Slide {
        &SLIDES[self.selected_slide]
    }

    pub fn current_specimen(&self) -> &Specimen {
        &SPECIMENS[self.selected_specimen]
    }

    pub fn current_system(&self) -> &BodySystem {
        &BODY_SYSTEMS[self.selected_system]
    }

    pub fn get_current_topic(&self) -> BiologyTopic {
        match self.view {
            BiolabView::Microscope => self.current_slide().topic,
            BiolabView::Dissection => self.current_specimen().topic,
            BiolabView::Anatomy => self.current_system().topic,
            BiolabView::DnaLab => BiologyTopic::DnaStructure,
            _ => BiologyTopic::CellStructure,
        }
    }
}

// =============================================================================
// QUIZ PARSING
// =============================================================================

#[derive(Deserialize)]
struct ParsedQuestion {
    question: String,
    options: [String; 4],
    correct_index: usize,
    explanation: String,
}

fn parse_quiz_response(response: &str, topic: BiologyTopic) -> Result<Vec<BiologyQuestion>, ()> {
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: Vec<ParsedQuestion> = serde_json::from_str(json_str).map_err(|_| ())?;

    Ok(parsed
        .into_iter()
        .map(|p| BiologyQuestion {
            question: p.question,
            options: p.options,
            correct_index: p.correct_index.min(3),
            explanation: p.explanation,
            topic,
        })
        .collect())
}

// =============================================================================
// GAME ENGINE
// =============================================================================

impl GameEngine for BiolabState {
    fn tick(&mut self) {
        if self.pending_generation {
            self.generate_quiz();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            BiolabView::LabMenu => self.handle_menu_key(key),
            BiolabView::Microscope => self.handle_microscope_key(key),
            BiolabView::DnaLab => self.handle_dna_key(key),
            BiolabView::Dissection => self.handle_dissection_key(key),
            BiolabView::Anatomy => self.handle_anatomy_key(key),
            BiolabView::Quiz => self.handle_quiz_key(key),
            BiolabView::QuizFeedback => self.handle_feedback_key(key),
            BiolabView::QuizResults => self.handle_results_key(key),
            BiolabView::Progress => self.handle_progress_key(key),
            BiolabView::Loading => KeyHandleResult::Handled,
            BiolabView::Error => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                    self.view = BiolabView::LabMenu;
                    self.error_message = None;
                }
                KeyHandleResult::Handled
            }
        }
    }

    fn get_score(&self) -> u32 {
        self.total_score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        // Win condition: master at least 5 topics
        self.topics_mastered.len() >= 5
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.topics_mastered.len() as u32)
    }
}

impl BiolabState {
    fn handle_menu_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_lab > 0 {
                    self.selected_lab -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_lab < LabType::all().len() - 1 {
                    self.selected_lab += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let lab = LabType::all()[self.selected_lab];
                self.enter_lab(lab);
                KeyHandleResult::Handled
            }
            KeyCode::Char('1') => {
                self.enter_lab(LabType::Microscope);
                KeyHandleResult::Handled
            }
            KeyCode::Char('2') => {
                self.enter_lab(LabType::DnaLab);
                KeyHandleResult::Handled
            }
            KeyCode::Char('3') => {
                self.enter_lab(LabType::Dissection);
                KeyHandleResult::Handled
            }
            KeyCode::Char('4') => {
                self.enter_lab(LabType::Anatomy);
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.view = BiolabView::Progress;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_microscope_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.back_to_menu();
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_slide > 0 {
                    self.selected_slide -= 1;
                    self.selected_organelle = 0;
                    self.mark_slide_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_slide < SLIDES.len() - 1 {
                    self.selected_slide += 1;
                    self.selected_organelle = 0;
                    self.mark_slide_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let slide = &SLIDES[self.selected_slide];
                if self.selected_organelle > 0 {
                    self.selected_organelle -= 1;
                } else {
                    self.selected_organelle = slide.organelles.len().saturating_sub(1);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let slide = &SLIDES[self.selected_slide];
                if self.selected_organelle < slide.organelles.len() - 1 {
                    self.selected_organelle += 1;
                } else {
                    self.selected_organelle = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                let topic = self.current_slide().topic;
                self.start_quiz(topic);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_dna_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                if self.dna_mode != DnaLabMode::Helix {
                    self.dna_mode = DnaLabMode::Helix;
                } else {
                    self.back_to_menu();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.dna_mode = match self.dna_mode {
                    DnaLabMode::Helix => DnaLabMode::Build,
                    DnaLabMode::Build => DnaLabMode::Transcription,
                    DnaLabMode::Transcription => DnaLabMode::Helix,
                };
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.dna_mode == DnaLabMode::Build && self.dna_cursor > 0 {
                    self.dna_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.dna_mode == DnaLabMode::Build
                    && self.dna_cursor < self.dna_sequence.len() - 1
                {
                    self.dna_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if self.dna_mode == DnaLabMode::Build {
                    self.dna_sequence[self.dna_cursor] = DnaBase::Adenine;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if self.dna_mode == DnaLabMode::Build {
                    self.dna_sequence[self.dna_cursor] = DnaBase::Thymine;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                if self.dna_mode == DnaLabMode::Build {
                    self.dna_sequence[self.dna_cursor] = DnaBase::Guanine;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if self.dna_mode == DnaLabMode::Build {
                    self.dna_sequence[self.dna_cursor] = DnaBase::Cytosine;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.start_quiz(BiologyTopic::DnaStructure);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_dissection_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.back_to_menu();
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_specimen > 0 {
                    self.selected_specimen -= 1;
                    self.selected_part = 0;
                    self.mark_specimen_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_specimen < SPECIMENS.len() - 1 {
                    self.selected_specimen += 1;
                    self.selected_part = 0;
                    self.mark_specimen_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let specimen = &SPECIMENS[self.selected_specimen];
                if self.selected_part > 0 {
                    self.selected_part -= 1;
                } else {
                    self.selected_part = specimen.parts.len().saturating_sub(1);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let specimen = &SPECIMENS[self.selected_specimen];
                if self.selected_part < specimen.parts.len() - 1 {
                    self.selected_part += 1;
                } else {
                    self.selected_part = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                let topic = self.current_specimen().topic;
                self.start_quiz(topic);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_anatomy_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.back_to_menu();
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_system > 0 {
                    self.selected_system -= 1;
                    self.selected_part = 0;
                    self.mark_system_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_system < BODY_SYSTEMS.len() - 1 {
                    self.selected_system += 1;
                    self.selected_part = 0;
                    self.mark_system_viewed();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let system = &BODY_SYSTEMS[self.selected_system];
                if self.selected_part > 0 {
                    self.selected_part -= 1;
                } else {
                    self.selected_part = system.parts.len().saturating_sub(1);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let system = &BODY_SYSTEMS[self.selected_system];
                if self.selected_part < system.parts.len() - 1 {
                    self.selected_part += 1;
                } else {
                    self.selected_part = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                let topic = self.current_system().topic;
                self.start_quiz(topic);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_quiz_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.view = self.previous_view;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_answer > 0 {
                    self.selected_answer -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_answer < 3 {
                    self.selected_answer += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.answer_question(self.selected_answer);
                KeyHandleResult::Handled
            }
            KeyCode::Char('1') | KeyCode::Char('a') | KeyCode::Char('A') => {
                self.answer_question(0);
                KeyHandleResult::Handled
            }
            KeyCode::Char('2') | KeyCode::Char('b') | KeyCode::Char('B') => {
                self.answer_question(1);
                KeyHandleResult::Handled
            }
            KeyCode::Char('3') | KeyCode::Char('c') | KeyCode::Char('C') => {
                self.answer_question(2);
                KeyHandleResult::Handled
            }
            KeyCode::Char('4') | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.answer_question(3);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_feedback_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.next_question();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.view = self.previous_view;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.view = self.previous_view;
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Retry quiz
                self.start_quiz(self.quiz_topic);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_progress_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.view = BiolabView::LabMenu;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}
