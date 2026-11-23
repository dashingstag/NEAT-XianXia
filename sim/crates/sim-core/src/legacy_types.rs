//! Frozen serde layouts shared by the schema 6-13 readers (#54).
//!
//! Each `Legacy*` type below is a copy of the live type's layout as those
//! saves wrote it. Field order, field types and enum variant order are what
//! postcard decodes, so none of them may change. Live types can now evolve
//! freely; the `From` impls are the only place that has to follow them.
//! `legacy_schema` pins every reader's full traced schema, so a live type
//! that a reader still embeds cannot change without a test failing.
//!
//! The `From<live>` impls exist only for tests that synthesize old saves
//! from a current world.
use super::*;

/// Goods count when schemas 6-13 were written; live arrays are padded to
/// `GOODS` on load.
const V13_GOODS: usize = 13;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LegacyAspiration {
    Wealth,
    Blacksmith,
    Family,
    Farmer,
    Service,
    Teacher,
    Homeowner,
    Shopkeeper,
    Artisan,
    Hunter,
    Haulier,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LegacyGoalKind {
    Save,
    Job,
    Craft,
    Farm,
    Marry,
    Rest,
    Eat,
    Trade,
    Tool,
    Child,
    Harvest,
    Teach,
    OwnHome,
    LeaseShop,
    Produce,
    Transport,
    Hunt,
    BusinessLoan,
    Weapon,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyGoal {
    pub kind: LegacyGoalKind,
    pub target: u64,
    pub expires: u64,
    pub created: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyPregnancy {
    pub father: u32,
    pub due: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyPerson {
    pub given: u16,
    pub surname: u16,
    pub gender: u8,
    pub initial_age_days: u32,
    pub born: u64,
    pub father: u32,
    pub mother: u32,
    pub spouse: u32,
    pub children: Vec<u32>,
    pub pregnancy: Option<LegacyPregnancy>,
    pub last_birth: u64,
    pub aspiration: LegacyAspiration,
    pub aspiration_done: bool,
    pub goals: Vec<LegacyGoal>,
    pub happiness: f32,
    pub meals: u64,
    pub trades: u64,
    pub jobs: u64,
    pub crafts: u64,
    pub farm_shifts: u64,
    pub harvested: u64,
    pub rest_cycles: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyPolicyChoice {
    pub action: String,
    pub score: f32,
    pub inputs: Vec<f32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyCouncil {
    pub genome: Genome,
    pub innovations: Innovations,
    pub graveyard: u32,
    pub near_dead: u32,
    pub remote_dead: u32,
    pub vacancies: u32,
    pub reward: u64,
    pub escrow: u64,
    pub builder_open: bool,
    pub builder_escrow: u64,
    pub last_filled: u64,
    pub last_revenue: u64,
    pub last_expenses: u64,
    pub income_rate: f32,
    pub expense_rate: f32,
    pub candidates: Vec<u32>,
    pub last_action: String,
    pub blocked: String,
    pub trace: Vec<LegacyPolicyChoice>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyLedger {
    pub initial: [u64; V13_GOODS],
    pub renewed: [u64; V13_GOODS],
    pub consumed: [u64; V13_GOODS],
    pub built: [u64; V13_GOODS],
    pub initial_coins: u64,
    pub minted_coins: u64,
    pub minted_ore: [u64; V13_GOODS],
    pub crafted_in: [u64; V13_GOODS],
    pub crafted_out: [u64; V13_GOODS],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyStatistics {
    pub decisions: u64,
    pub trades: u64,
    pub shipments: u64,
    pub harvested: u64,
    pub deaths: u64,
    pub buildings: u64,
    pub roads: u64,
    pub rejected: u64,
    pub convoys_departed: u64,
    pub solo_departures: u64,
    pub camps_founded: u64,
    pub barter_trades: u64,
    pub paid_services: u64,
    pub food_services: u64,
    pub mintings: u64,
    pub burials: u64,
    pub graveyards_built: u64,
    pub jobs_completed: u64,
    pub jobs_cancelled: u64,
    pub births: u64,
    pub marriages: u64,
    pub goals_completed: u64,
    pub tools_crafted: u64,
    pub farm_shifts: u64,
    pub rice_harvested: u64,
}

/// Maps between two enums whose variants share names, in both directions.
macro_rules! same_variants {
    ($legacy:ident, $live:path, [$($v:ident),* $(,)?]) => {
        impl From<$legacy> for $live {
            fn from(old: $legacy) -> Self {
                match old { $($legacy::$v => Self::$v,)* }
            }
        }
        #[cfg(test)]
        impl From<$live> for $legacy {
            fn from(live: $live) -> Self {
                use $live as Live;
                match live { $(Live::$v => Self::$v,)* }
            }
        }
    };
}
same_variants!(
    LegacyAspiration,
    life::Aspiration,
    [
        Wealth, Blacksmith, Family, Farmer, Service, Teacher, Homeowner, Shopkeeper, Artisan,
        Hunter, Haulier,
    ]
);
same_variants!(
    LegacyGoalKind,
    life::GoalKind,
    [
        Save,
        Job,
        Craft,
        Farm,
        Marry,
        Rest,
        Eat,
        Trade,
        Tool,
        Child,
        Harvest,
        Teach,
        OwnHome,
        LeaseShop,
        Produce,
        Transport,
        Hunt,
        BusinessLoan,
        Weapon,
    ]
);

/// Field-by-field moves between a frozen struct and its live type, plus the
/// test-only reverse. Each field converts with `.into()`, so nested frozen
/// types convert too. A field added to the live type stops this compiling
/// until the conversion says how to fill it.
macro_rules! twin {
    ($legacy:ident, $live:path, [$($f:ident),* $(,)?]) => {
        impl From<$legacy> for $live {
            fn from(old: $legacy) -> Self {
                Self { $($f: old.$f.into(),)* }
            }
        }
        #[cfg(test)]
        impl From<$live> for $legacy {
            fn from(live: $live) -> Self {
                Self { $($f: live.$f.into(),)* }
            }
        }
    };
}

twin!(LegacyGoal, life::Goal, [kind, target, expires, created]);
twin!(LegacyPregnancy, life::Pregnancy, [father, due]);

impl From<LegacyPerson> for Person {
    fn from(old: LegacyPerson) -> Self {
        Self {
            given: old.given,
            surname: old.surname,
            gender: old.gender,
            initial_age_days: old.initial_age_days,
            born: old.born,
            father: old.father,
            mother: old.mother,
            spouse: old.spouse,
            children: old.children,
            pregnancy: old.pregnancy.map(Into::into),
            last_birth: old.last_birth,
            aspiration: old.aspiration.into(),
            aspiration_done: old.aspiration_done,
            goals: old.goals.into_iter().map(Into::into).collect(),
            happiness: old.happiness,
            meals: old.meals,
            trades: old.trades,
            jobs: old.jobs,
            crafts: old.crafts,
            farm_shifts: old.farm_shifts,
            harvested: old.harvested,
            rest_cycles: old.rest_cycles,
        }
    }
}
#[cfg(test)]
impl From<Person> for LegacyPerson {
    fn from(p: Person) -> Self {
        Self {
            given: p.given,
            surname: p.surname,
            gender: p.gender,
            initial_age_days: p.initial_age_days,
            born: p.born,
            father: p.father,
            mother: p.mother,
            spouse: p.spouse,
            children: p.children,
            pregnancy: p.pregnancy.map(Into::into),
            last_birth: p.last_birth,
            aspiration: p.aspiration.into(),
            aspiration_done: p.aspiration_done,
            goals: p.goals.into_iter().map(Into::into).collect(),
            happiness: p.happiness,
            meals: p.meals,
            trades: p.trades,
            jobs: p.jobs,
            crafts: p.crafts,
            farm_shifts: p.farm_shifts,
            harvested: p.harvested,
            rest_cycles: p.rest_cycles,
        }
    }
}

twin!(
    LegacyPolicyChoice,
    civic::PolicyChoice,
    [action, score, inputs]
);

impl From<LegacyCouncil> for Council {
    fn from(old: LegacyCouncil) -> Self {
        Self {
            genome: old.genome,
            innovations: old.innovations,
            brain: None,
            graveyard: old.graveyard,
            near_dead: old.near_dead,
            remote_dead: old.remote_dead,
            vacancies: old.vacancies,
            reward: old.reward,
            escrow: old.escrow,
            builder_open: old.builder_open,
            builder_escrow: old.builder_escrow,
            last_filled: old.last_filled,
            last_revenue: old.last_revenue,
            last_expenses: old.last_expenses,
            income_rate: old.income_rate,
            expense_rate: old.expense_rate,
            candidates: old.candidates,
            last_action: old.last_action,
            blocked: old.blocked,
            trace: old.trace.into_iter().map(Into::into).collect(),
        }
    }
}
#[cfg(test)]
impl From<Council> for LegacyCouncil {
    fn from(c: Council) -> Self {
        Self {
            genome: c.genome,
            innovations: c.innovations,
            graveyard: c.graveyard,
            near_dead: c.near_dead,
            remote_dead: c.remote_dead,
            vacancies: c.vacancies,
            reward: c.reward,
            escrow: c.escrow,
            builder_open: c.builder_open,
            builder_escrow: c.builder_escrow,
            last_filled: c.last_filled,
            last_revenue: c.last_revenue,
            last_expenses: c.last_expenses,
            income_rate: c.income_rate,
            expense_rate: c.expense_rate,
            candidates: c.candidates,
            last_action: c.last_action,
            blocked: c.blocked,
            trace: c.trace.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<LegacyLedger> for Ledger {
    fn from(old: LegacyLedger) -> Self {
        Self {
            initial: legacy_v5::pad(old.initial),
            renewed: legacy_v5::pad(old.renewed),
            consumed: legacy_v5::pad(old.consumed),
            built: legacy_v5::pad(old.built),
            initial_coins: old.initial_coins,
            minted_coins: old.minted_coins,
            minted_ore: legacy_v5::pad(old.minted_ore),
            crafted_in: legacy_v5::pad(old.crafted_in),
            crafted_out: legacy_v5::pad(old.crafted_out),
        }
    }
}
#[cfg(test)]
impl From<Ledger> for LegacyLedger {
    fn from(l: Ledger) -> Self {
        let cut = |a: [u64; GOODS]| std::array::from_fn(|k| a[k]);
        Self {
            initial: cut(l.initial),
            renewed: cut(l.renewed),
            consumed: cut(l.consumed),
            built: cut(l.built),
            initial_coins: l.initial_coins,
            minted_coins: l.minted_coins,
            minted_ore: cut(l.minted_ore),
            crafted_in: cut(l.crafted_in),
            crafted_out: cut(l.crafted_out),
        }
    }
}

twin!(
    LegacyStatistics,
    Statistics,
    [
        decisions,
        trades,
        shipments,
        harvested,
        deaths,
        buildings,
        roads,
        rejected,
        convoys_departed,
        solo_departures,
        camps_founded,
        barter_trades,
        paid_services,
        food_services,
        mintings,
        burials,
        graveyards_built,
        jobs_completed,
        jobs_cancelled,
        births,
        marriages,
        goals_completed,
        tools_crafted,
        farm_shifts,
        rice_harvested,
    ]
);
