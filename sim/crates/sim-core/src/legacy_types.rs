//! Frozen serde layouts that the schema 4-13 readers decode old saves into (#54).
//!
//! Each `Legacy*` type is a copy of a live type's layout as those saves wrote
//! it. Postcard decodes by position, so field order, field types, array
//! lengths and enum variant order must never change here. No reader decodes
//! into a live type any more, so live types can evolve freely; only the `From`
//! conversions below have to follow them, and they stop compiling until they
//! do. `legacy_schema` pins every reader's traced schema against
//! `legacy_schema.json`.
//!
//! Batches 2-4 were generated from the traced v13 schema: each live struct's
//! field list had to match the trace exactly, and serde-skipped runtime
//! fields are rebuilt as defaults on load, exactly as serde did before.
//!
//! The reverse `From<live>` conversions exist for tests that synthesize old
//! saves from a current world; `LegacyAction`'s also serves
//! `From<Agents> for LegacyAgents`.
use super::*;
use std::collections::BTreeMap;

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
    pub genome: LegacyGenome,
    pub innovations: LegacyInnovations,
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
        same_variants!(@from $legacy, $live, [$($v),*]);
        #[cfg(test)]
        same_variants!(@back $legacy, $live, [$($v),*]);
    };
    // Reverse conversion outside tests too (a reader builds this from live data).
    (both $legacy:ident, $live:path, [$($v:ident),* $(,)?]) => {
        same_variants!(@from $legacy, $live, [$($v),*]);
        same_variants!(@back $legacy, $live, [$($v),*]);
    };
    (@from $legacy:ident, $live:path, [$($v:ident),*]) => {
        impl From<$legacy> for $live {
            fn from(old: $legacy) -> Self {
                match old {
                    $($legacy::$v => Self::$v,)*
                }
            }
        }
    };
    (@back $legacy:ident, $live:path, [$($v:ident),*]) => {
        impl From<$live> for $legacy {
            fn from(live: $live) -> Self {
                use $live as Live;
                match live {
                    $(Live::$v => Self::$v,)*
                }
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
            genome: old.genome.into(),
            innovations: old.innovations.into(),
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
            genome: c.genome.into(),
            innovations: c.innovations.into(),
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

// Batch 2: urban, banking, education, agriculture and taverns (generated
// from the traced v13 schema; each field list matched the live struct).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyUrban {
    pub states: Vec<LegacyTownState>,
    pub zones: Vec<LegacyZone>,
    pub buildings: Vec<LegacyBuilding>,
    pub jobs: Vec<Option<LegacyJob>>,
    pub assigned: Vec<u32>,
    pub owned: BTreeMap<(u32, u8), u64>,
    pub progress: BTreeMap<u32, LegacyProgress>,
    pub estates: Vec<LegacyEstate>,
    pub produced: [u64; 5],
    pub consumed: [u64; 5],
    pub cleared: Vec<(f32, f32, f32)>,
    pub evicted_until: BTreeMap<u32, u64>,
    pub posted_public: u64,
    pub posted_private: u64,
    pub completed: u64,
    pub evictions: u64,
    pub retail_sales: BTreeMap<(u32, u8), u64>,
}
impl From<LegacyUrban> for urban::Urban {
    fn from(old: LegacyUrban) -> Self {
        Self {
            states: old.states.into_iter().map(Into::into).collect(),
            zones: old.zones.into_iter().map(Into::into).collect(),
            buildings: old.buildings.into_iter().map(Into::into).collect(),
            jobs: old.jobs.into_iter().map(|o| o.map(Into::into)).collect(),
            assigned: old.assigned,
            owned: old.owned,
            progress: old
                .progress
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            estates: old.estates.into_iter().map(Into::into).collect(),
            produced: old.produced,
            consumed: old.consumed,
            cleared: old.cleared,
            evicted_until: old.evicted_until,
            posted_public: old.posted_public,
            posted_private: old.posted_private,
            completed: old.completed,
            evictions: old.evictions,
            retail_sales: old.retail_sales,
        }
    }
}
#[cfg(test)]
impl From<urban::Urban> for LegacyUrban {
    fn from(live: urban::Urban) -> Self {
        Self {
            states: live.states.into_iter().map(Into::into).collect(),
            zones: live.zones.into_iter().map(Into::into).collect(),
            buildings: live.buildings.into_iter().map(Into::into).collect(),
            jobs: live.jobs.into_iter().map(|o| o.map(Into::into)).collect(),
            assigned: live.assigned,
            owned: live.owned,
            progress: live
                .progress
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            estates: live.estates.into_iter().map(Into::into).collect(),
            produced: live.produced,
            consumed: live.consumed,
            cleared: live.cleared,
            evicted_until: live.evicted_until,
            posted_public: live.posted_public,
            posted_private: live.posted_private,
            completed: live.completed,
            evictions: live.evictions,
            retail_sales: live.retail_sales,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyTownState {
    pub zoned: bool,
    pub persona: u8,
    pub last_eviction: u64,
}
impl From<LegacyTownState> for urban::TownState {
    fn from(old: LegacyTownState) -> Self {
        Self {
            zoned: old.zoned,
            persona: old.persona,
            last_eviction: old.last_eviction,
        }
    }
}
#[cfg(test)]
impl From<urban::TownState> for LegacyTownState {
    fn from(live: urban::TownState) -> Self {
        Self {
            zoned: live.zoned,
            persona: live.persona,
            last_eviction: live.last_eviction,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyZone {
    pub market: u32,
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
impl From<LegacyZone> for urban::Zone {
    fn from(old: LegacyZone) -> Self {
        Self {
            market: old.market,
            kind: old.kind,
            x: old.x,
            y: old.y,
            radius: old.radius,
        }
    }
}
#[cfg(test)]
impl From<urban::Zone> for LegacyZone {
    fn from(live: urban::Zone) -> Self {
        Self {
            market: live.market,
            kind: live.kind,
            x: live.x,
            y: live.y,
            radius: live.radius,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyBuilding {
    pub market: u32,
    pub kind: u8,
    pub line: u8,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub needs_clear: bool,
    pub advertised: bool,
    pub owner: u32,
    pub renter: u32,
    pub price: u64,
    pub rent: u64,
    pub leased: u64,
    pub arrears: u64,
    pub last_rent: u64,
    pub shelter: u32,
    pub cash: u64,
    pub raw: [u64; 13],
    pub wares: [u64; 5],
    pub made: u64,
    pub sold: u64,
    pub last_service: u64,
    pub last_open: u64,
}
impl From<LegacyBuilding> for urban::Building {
    fn from(old: LegacyBuilding) -> Self {
        Self {
            market: old.market,
            kind: old.kind,
            line: old.line,
            x: old.x,
            y: old.y,
            ready: old.ready,
            progress: old.progress,
            needs_clear: old.needs_clear,
            advertised: old.advertised,
            owner: old.owner,
            renter: old.renter,
            price: old.price,
            rent: old.rent,
            leased: old.leased,
            arrears: old.arrears,
            last_rent: old.last_rent,
            shelter: old.shelter,
            cash: old.cash,
            raw: old.raw,
            wares: old.wares,
            made: old.made,
            sold: old.sold,
            last_service: old.last_service,
            last_open: old.last_open,
        }
    }
}
#[cfg(test)]
impl From<urban::Building> for LegacyBuilding {
    fn from(live: urban::Building) -> Self {
        Self {
            market: live.market,
            kind: live.kind,
            line: live.line,
            x: live.x,
            y: live.y,
            ready: live.ready,
            progress: live.progress,
            needs_clear: live.needs_clear,
            advertised: live.advertised,
            owner: live.owner,
            renter: live.renter,
            price: live.price,
            rent: live.rent,
            leased: live.leased,
            arrears: live.arrears,
            last_rent: live.last_rent,
            shelter: live.shelter,
            cash: live.cash,
            raw: live.raw,
            wares: live.wares,
            made: live.made,
            sold: live.sold,
            last_service: live.last_service,
            last_open: live.last_open,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyJob {
    pub market: u32,
    pub kind: u8,
    pub payer: u32,
    pub firm: u32,
    pub source: u32,
    pub target: u32,
    pub worker: u32,
    pub phase: u8,
    pub x: f32,
    pub y: f32,
    pub raw: [u64; 13],
    pub wares: [u64; 5],
    pub reward: u64,
    pub progress: f32,
}
impl From<LegacyJob> for urban::Job {
    fn from(old: LegacyJob) -> Self {
        Self {
            market: old.market,
            kind: old.kind,
            payer: old.payer,
            firm: old.firm,
            source: old.source,
            target: old.target,
            worker: old.worker,
            phase: old.phase,
            x: old.x,
            y: old.y,
            raw: old.raw,
            wares: old.wares,
            reward: old.reward,
            progress: old.progress,
        }
    }
}
#[cfg(test)]
impl From<urban::Job> for LegacyJob {
    fn from(live: urban::Job) -> Self {
        Self {
            market: live.market,
            kind: live.kind,
            payer: live.payer,
            firm: live.firm,
            source: live.source,
            target: live.target,
            worker: live.worker,
            phase: live.phase,
            x: live.x,
            y: live.y,
            raw: live.raw,
            wares: live.wares,
            reward: live.reward,
            progress: live.progress,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyEstate {
    pub owner: u32,
    pub x: f32,
    pub y: f32,
    pub cash: u64,
    pub raw: [u64; 13],
    pub wares: [u64; 5],
}
impl From<LegacyEstate> for urban::Estate {
    fn from(old: LegacyEstate) -> Self {
        Self {
            owner: old.owner,
            x: old.x,
            y: old.y,
            cash: old.cash,
            raw: old.raw,
            wares: old.wares,
        }
    }
}
#[cfg(test)]
impl From<urban::Estate> for LegacyEstate {
    fn from(live: urban::Estate) -> Self {
        Self {
            owner: live.owner,
            x: live.x,
            y: live.y,
            cash: live.cash,
            raw: live.raw,
            wares: live.wares,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyProgress {
    pub made: u64,
    pub hauled: u64,
    pub hunted: u64,
    pub sales: u64,
}
impl From<LegacyProgress> for urban::Progress {
    fn from(old: LegacyProgress) -> Self {
        Self {
            made: old.made,
            hauled: old.hauled,
            hunted: old.hunted,
            sales: old.sales,
        }
    }
}
#[cfg(test)]
impl From<urban::Progress> for LegacyProgress {
    fn from(live: urban::Progress) -> Self {
        Self {
            made: live.made,
            hauled: live.hauled,
            hunted: live.hunted,
            sales: live.sales,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyBanking {
    pub banks: Vec<LegacyBank>,
    pub loans: BTreeMap<u32, LegacyLoan>,
    pub approved: BTreeMap<u32, u64>,
    pub interest_paid: u64,
    pub principal_paid: u64,
}
impl From<LegacyBanking> for banking::Banking {
    fn from(old: LegacyBanking) -> Self {
        Self {
            banks: old.banks.into_iter().map(Into::into).collect(),
            loans: old.loans.into_iter().map(|(k, v)| (k, v.into())).collect(),
            approved: old.approved,
            interest_paid: old.interest_paid,
            principal_paid: old.principal_paid,
        }
    }
}
#[cfg(test)]
impl From<banking::Banking> for LegacyBanking {
    fn from(live: banking::Banking) -> Self {
        Self {
            banks: live.banks.into_iter().map(Into::into).collect(),
            loans: live.loans.into_iter().map(|(k, v)| (k, v.into())).collect(),
            approved: live.approved,
            interest_paid: live.interest_paid,
            principal_paid: live.principal_paid,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyBank {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub cash: u64,
    pub builder_escrow: u64,
    pub worker: u32,
    pub rate: u8,
    pub next_review: u64,
    pub sales_at_review: u64,
    pub last_year_sales: u64,
}
impl From<LegacyBank> for banking::Bank {
    fn from(old: LegacyBank) -> Self {
        Self {
            market: old.market,
            x: old.x,
            y: old.y,
            ready: old.ready,
            progress: old.progress,
            cash: old.cash,
            builder_escrow: old.builder_escrow,
            worker: old.worker,
            rate: old.rate,
            next_review: old.next_review,
            sales_at_review: old.sales_at_review,
            last_year_sales: old.last_year_sales,
        }
    }
}
#[cfg(test)]
impl From<banking::Bank> for LegacyBank {
    fn from(live: banking::Bank) -> Self {
        Self {
            market: live.market,
            x: live.x,
            y: live.y,
            ready: live.ready,
            progress: live.progress,
            cash: live.cash,
            builder_escrow: live.builder_escrow,
            worker: live.worker,
            rate: live.rate,
            next_review: live.next_review,
            sales_at_review: live.sales_at_review,
            last_year_sales: live.last_year_sales,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyLoan {
    pub bank: u32,
    pub principal: u64,
    pub interest: u64,
    pub remainder: u64,
    pub rate: u8,
    pub opened: u64,
    pub last_accrual: u64,
    pub next_reprice: u64,
}
impl From<LegacyLoan> for banking::Loan {
    fn from(old: LegacyLoan) -> Self {
        Self {
            bank: old.bank,
            principal: old.principal,
            interest: old.interest,
            remainder: old.remainder,
            rate: old.rate,
            opened: old.opened,
            last_accrual: old.last_accrual,
            next_reprice: old.next_reprice,
        }
    }
}
#[cfg(test)]
impl From<banking::Loan> for LegacyLoan {
    fn from(live: banking::Loan) -> Self {
        Self {
            bank: live.bank,
            principal: live.principal,
            interest: live.interest,
            remainder: live.remainder,
            rate: live.rate,
            opened: live.opened,
            last_accrual: live.last_accrual,
            next_reprice: live.next_reprice,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyEducation {
    pub schools: Vec<LegacySchool>,
    pub attendance: Vec<u16>,
    pub teaching_shifts: Vec<u64>,
    pub children: Vec<u32>,
    pub pupils: Vec<u32>,
    pub uneducated_children: Vec<u32>,
    pub bonus_claims: BTreeMap<(u32, u32), u64>,
    pub graduated: u64,
    pub lessons: u64,
    pub bonuses_paid: u64,
}
impl From<LegacyEducation> for education::Education {
    fn from(old: LegacyEducation) -> Self {
        Self {
            schools: old.schools.into_iter().map(Into::into).collect(),
            attendance: old.attendance,
            teaching_shifts: old.teaching_shifts,
            children: old.children,
            pupils: old.pupils,
            uneducated_children: old.uneducated_children,
            bonus_claims: old.bonus_claims,
            graduated: old.graduated,
            lessons: old.lessons,
            bonuses_paid: old.bonuses_paid,
        }
    }
}
#[cfg(test)]
impl From<education::Education> for LegacyEducation {
    fn from(live: education::Education) -> Self {
        Self {
            schools: live.schools.into_iter().map(Into::into).collect(),
            attendance: live.attendance,
            teaching_shifts: live.teaching_shifts,
            children: live.children,
            pupils: live.pupils,
            uneducated_children: live.uneducated_children,
            bonus_claims: live.bonus_claims,
            graduated: live.graduated,
            lessons: live.lessons,
            bonuses_paid: live.bonuses_paid,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacySchool {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub build_escrow: u64,
    pub wage_escrow: u64,
    pub teacher: u32,
    pub last_taught: u64,
    pub session_day: u64,
    pub lesson_seconds: f32,
    pub students: BTreeMap<u32, f32>,
}
impl From<LegacySchool> for education::School {
    fn from(old: LegacySchool) -> Self {
        Self {
            market: old.market,
            x: old.x,
            y: old.y,
            ready: old.ready,
            progress: old.progress,
            build_escrow: old.build_escrow,
            wage_escrow: old.wage_escrow,
            teacher: old.teacher,
            last_taught: old.last_taught,
            session_day: old.session_day,
            lesson_seconds: old.lesson_seconds,
            students: old.students,
        }
    }
}
#[cfg(test)]
impl From<education::School> for LegacySchool {
    fn from(live: education::School) -> Self {
        Self {
            market: live.market,
            x: live.x,
            y: live.y,
            ready: live.ready,
            progress: live.progress,
            build_escrow: live.build_escrow,
            wage_escrow: live.wage_escrow,
            teacher: live.teacher,
            last_taught: live.last_taught,
            session_day: live.session_day,
            lesson_seconds: live.lesson_seconds,
            students: live.students,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyFarm {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub fertility: f32,
    pub moisture: f32,
    pub planted: u64,
    pub tended_days: u32,
    pub last_tended: u64,
    pub mature: bool,
    pub harvest_remaining: u64,
    pub yield_per_crop: u64,
    pub escrow: u64,
}
impl From<LegacyFarm> for agriculture::Farm {
    fn from(old: LegacyFarm) -> Self {
        Self {
            market: old.market,
            x: old.x,
            y: old.y,
            fertility: old.fertility,
            moisture: old.moisture,
            planted: old.planted,
            tended_days: old.tended_days,
            last_tended: old.last_tended,
            mature: old.mature,
            harvest_remaining: old.harvest_remaining,
            yield_per_crop: old.yield_per_crop,
            escrow: old.escrow,
        }
    }
}
#[cfg(test)]
impl From<agriculture::Farm> for LegacyFarm {
    fn from(live: agriculture::Farm) -> Self {
        Self {
            market: live.market,
            x: live.x,
            y: live.y,
            fertility: live.fertility,
            moisture: live.moisture,
            planted: live.planted,
            tended_days: live.tended_days,
            last_tended: live.last_tended,
            mature: live.mature,
            harvest_remaining: live.harvest_remaining,
            yield_per_crop: live.yield_per_crop,
            escrow: live.escrow,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyFertileRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fertility: f32,
    pub moisture: f32,
}
impl From<LegacyFertileRegion> for agriculture::FertileRegion {
    fn from(old: LegacyFertileRegion) -> Self {
        Self {
            x: old.x,
            y: old.y,
            width: old.width,
            height: old.height,
            fertility: old.fertility,
            moisture: old.moisture,
        }
    }
}
#[cfg(test)]
impl From<agriculture::FertileRegion> for LegacyFertileRegion {
    fn from(live: agriculture::FertileRegion) -> Self {
        Self {
            x: live.x,
            y: live.y,
            width: live.width,
            height: live.height,
            fertility: live.fertility,
            moisture: live.moisture,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyTavern {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub escrow: u64,
    pub visitors: u32,
    pub guests: Vec<u32>,
}
impl From<LegacyTavern> for social::Tavern {
    fn from(old: LegacyTavern) -> Self {
        Self {
            market: old.market,
            x: old.x,
            y: old.y,
            ready: old.ready,
            progress: old.progress,
            escrow: old.escrow,
            visitors: old.visitors,
            guests: old.guests,
        }
    }
}
#[cfg(test)]
impl From<social::Tavern> for LegacyTavern {
    fn from(live: social::Tavern) -> Self {
        Self {
            market: live.market,
            x: live.x,
            y: live.y,
            ready: live.ready,
            progress: live.progress,
            escrow: live.escrow,
            visitors: live.visitors,
            guests: live.guests,
        }
    }
}

// Batch 3: wildlife, world fixtures, civic records and relationships
// (generated from the traced v13 schema; each field list matched the live
// struct, serde-skipped runtime fields are rebuilt as defaults on load).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LegacyJobKind {
    Build,
    Haul,
}
same_variants!(LegacyJobKind, civic::JobKind, [Build, Haul]);
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyWildlife {
    pub forests: Vec<LegacyForest>,
    pub grass: Vec<LegacyGrass>,
    pub animals: Vec<LegacyAnimal>,
    pub innovations: LegacyInnovations,
    pub rng: LegacyRng,
    pub ledger: LegacyEcologyLedger,
}
impl From<LegacyWildlife> for wildlife::Wildlife {
    fn from(old: LegacyWildlife) -> Self {
        Self {
            forests: old.forests.into_iter().map(Into::into).collect(),
            grass: old.grass.into_iter().map(Into::into).collect(),
            animals: old.animals.into_iter().map(Into::into).collect(),
            innovations: old.innovations.into(),
            rng: old.rng.into(),
            ledger: old.ledger.into(),
            spawns: Default::default(),
            spears: Default::default(),
            bows: Default::default(),
            hunting_stats: Default::default(),
        }
    }
}
#[cfg(test)]
impl From<wildlife::Wildlife> for LegacyWildlife {
    fn from(live: wildlife::Wildlife) -> Self {
        Self {
            forests: live.forests.into_iter().map(Into::into).collect(),
            grass: live.grass.into_iter().map(Into::into).collect(),
            animals: live.animals.into_iter().map(Into::into).collect(),
            innovations: live.innovations.into(),
            rng: live.rng.into(),
            ledger: live.ledger.into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyAnimal {
    pub species: u8,
    pub female: bool,
    pub forest: u32,
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub alive: bool,
    pub health: f32,
    pub hunger: f32,
    pub energy: f32,
    pub born: u64,
    pub initial_age: u32,
    pub mother: u32,
    pub father: u32,
    pub pregnancy: Option<LegacyAnimalPregnancy>,
    pub last_mated: u64,
    pub action: u8,
    pub target: u32,
    pub goal_x: f32,
    pub goal_y: f32,
    pub timer: f32,
    pub threat: f32,
    pub threat_x: f32,
    pub threat_y: f32,
    pub genome: LegacyGenome,
}
impl From<LegacyAnimal> for wildlife::Animal {
    fn from(old: LegacyAnimal) -> Self {
        Self {
            species: old.species,
            female: old.female,
            forest: old.forest,
            x: old.x,
            y: old.y,
            heading: old.heading,
            alive: old.alive,
            health: old.health,
            hunger: old.hunger,
            energy: old.energy,
            born: old.born,
            initial_age: old.initial_age,
            mother: old.mother,
            father: old.father,
            pregnancy: old.pregnancy.map(Into::into),
            last_mated: old.last_mated,
            action: old.action,
            target: old.target,
            goal_x: old.goal_x,
            goal_y: old.goal_y,
            timer: old.timer,
            threat: old.threat,
            threat_x: old.threat_x,
            threat_y: old.threat_y,
            genome: old.genome.into(),
            brain: Default::default(),
        }
    }
}
#[cfg(test)]
impl From<wildlife::Animal> for LegacyAnimal {
    fn from(live: wildlife::Animal) -> Self {
        Self {
            species: live.species,
            female: live.female,
            forest: live.forest,
            x: live.x,
            y: live.y,
            heading: live.heading,
            alive: live.alive,
            health: live.health,
            hunger: live.hunger,
            energy: live.energy,
            born: live.born,
            initial_age: live.initial_age,
            mother: live.mother,
            father: live.father,
            pregnancy: live.pregnancy.map(Into::into),
            last_mated: live.last_mated,
            action: live.action,
            target: live.target,
            goal_x: live.goal_x,
            goal_y: live.goal_y,
            timer: live.timer,
            threat: live.threat,
            threat_x: live.threat_x,
            threat_y: live.threat_y,
            genome: live.genome.into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyAnimalPregnancy {
    pub father: u32,
    pub due: u64,
}
impl From<LegacyAnimalPregnancy> for wildlife::AnimalPregnancy {
    fn from(old: LegacyAnimalPregnancy) -> Self {
        Self {
            father: old.father,
            due: old.due,
        }
    }
}
#[cfg(test)]
impl From<wildlife::AnimalPregnancy> for LegacyAnimalPregnancy {
    fn from(live: wildlife::AnimalPregnancy) -> Self {
        Self {
            father: live.father,
            due: live.due,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LegacyEcologyLedger {
    pub initial_grass: u64,
    pub grown_grass: u64,
    pub eaten_grass: u64,
    pub eaten_berries: u64,
    pub initial_animals: u64,
    pub births: u64,
    pub deaths: u64,
    pub conceptions: u64,
    pub lost_pregnancies: u64,
}
impl From<LegacyEcologyLedger> for wildlife::EcologyLedger {
    fn from(old: LegacyEcologyLedger) -> Self {
        Self {
            initial_grass: old.initial_grass,
            grown_grass: old.grown_grass,
            eaten_grass: old.eaten_grass,
            eaten_berries: old.eaten_berries,
            initial_animals: old.initial_animals,
            births: old.births,
            deaths: old.deaths,
            conceptions: old.conceptions,
            lost_pregnancies: old.lost_pregnancies,
        }
    }
}
#[cfg(test)]
impl From<wildlife::EcologyLedger> for LegacyEcologyLedger {
    fn from(live: wildlife::EcologyLedger) -> Self {
        Self {
            initial_grass: live.initial_grass,
            grown_grass: live.grown_grass,
            eaten_grass: live.eaten_grass,
            eaten_berries: live.eaten_berries,
            initial_animals: live.initial_animals,
            births: live.births,
            deaths: live.deaths,
            conceptions: live.conceptions,
            lost_pregnancies: live.lost_pregnancies,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyForest {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
impl From<LegacyForest> for wildlife::Forest {
    fn from(old: LegacyForest) -> Self {
        Self {
            x: old.x,
            y: old.y,
            radius: old.radius,
        }
    }
}
#[cfg(test)]
impl From<wildlife::Forest> for LegacyForest {
    fn from(live: wildlife::Forest) -> Self {
        Self {
            x: live.x,
            y: live.y,
            radius: live.radius,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyGrass {
    pub forest: u32,
    pub x: f32,
    pub y: f32,
    pub stock: u64,
    pub capacity: u64,
    pub growth: f64,
}
impl From<LegacyGrass> for wildlife::Grass {
    fn from(old: LegacyGrass) -> Self {
        Self {
            forest: old.forest,
            x: old.x,
            y: old.y,
            stock: old.stock,
            capacity: old.capacity,
            growth: old.growth,
        }
    }
}
#[cfg(test)]
impl From<wildlife::Grass> for LegacyGrass {
    fn from(live: wildlife::Grass) -> Self {
        Self {
            forest: live.forest,
            x: live.x,
            y: live.y,
            stock: live.stock,
            capacity: live.capacity,
            growth: live.growth,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyPatch {
    pub id: u32,
    pub kind: usize,
    pub x: f32,
    pub y: f32,
    pub stock: u64,
    pub capacity: u64,
    pub renewal: f64,
    pub growth: f64,
    pub age: f32,
}
impl From<LegacyPatch> for crate::Patch {
    fn from(old: LegacyPatch) -> Self {
        Self {
            id: old.id,
            kind: old.kind,
            x: old.x,
            y: old.y,
            stock: old.stock,
            capacity: old.capacity,
            renewal: old.renewal,
            growth: old.growth,
            age: old.age,
        }
    }
}
#[cfg(test)]
impl From<crate::Patch> for LegacyPatch {
    fn from(live: crate::Patch) -> Self {
        Self {
            id: live.id,
            kind: live.kind,
            x: live.x,
            y: live.y,
            stock: live.stock,
            capacity: live.capacity,
            renewal: live.renewal,
            growth: live.growth,
            age: live.age,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyRoad {
    pub a: u32,
    pub b: u32,
    pub built: bool,
    pub under_construction: bool,
    pub traffic: u64,
}
impl From<LegacyRoad> for crate::Road {
    fn from(old: LegacyRoad) -> Self {
        Self {
            a: old.a,
            b: old.b,
            built: old.built,
            under_construction: old.under_construction,
            traffic: old.traffic,
        }
    }
}
#[cfg(test)]
impl From<crate::Road> for LegacyRoad {
    fn from(live: crate::Road) -> Self {
        Self {
            a: live.a,
            b: live.b,
            built: live.built,
            under_construction: live.under_construction,
            traffic: live.traffic,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyShelter {
    pub x: f32,
    pub y: f32,
    pub settlement: Option<u32>,
    pub beds: Vec<u32>,
    pub free: Vec<u32>,
}
impl From<LegacyShelter> for rest::Shelter {
    fn from(old: LegacyShelter) -> Self {
        Self {
            x: old.x,
            y: old.y,
            settlement: old.settlement,
            beds: old.beds,
            free: old.free,
        }
    }
}
#[cfg(test)]
impl From<rest::Shelter> for LegacyShelter {
    fn from(live: rest::Shelter) -> Self {
        Self {
            x: live.x,
            y: live.y,
            settlement: live.settlement,
            beds: live.beds,
            free: live.free,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyConvoy {
    pub members: Vec<u32>,
    pub x: f32,
    pub y: f32,
    pub goal_x: f32,
    pub goal_y: f32,
    pub created: u64,
    pub departed: bool,
}
impl From<LegacyConvoy> for travel::Convoy {
    fn from(old: LegacyConvoy) -> Self {
        Self {
            members: old.members,
            x: old.x,
            y: old.y,
            goal_x: old.goal_x,
            goal_y: old.goal_y,
            created: old.created,
            departed: old.departed,
        }
    }
}
#[cfg(test)]
impl From<travel::Convoy> for LegacyConvoy {
    fn from(live: travel::Convoy) -> Self {
        Self {
            members: live.members,
            x: live.x,
            y: live.y,
            goal_x: live.goal_x,
            goal_y: live.goal_y,
            created: live.created,
            departed: live.departed,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyCorpse {
    pub agent: u32,
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub death_tick: u64,
    pub status: u8,
    pub claim: u32,
}
impl From<LegacyCorpse> for civic::Corpse {
    fn from(old: LegacyCorpse) -> Self {
        Self {
            agent: old.agent,
            market: old.market,
            x: old.x,
            y: old.y,
            death_tick: old.death_tick,
            status: old.status,
            claim: old.claim,
        }
    }
}
#[cfg(test)]
impl From<civic::Corpse> for LegacyCorpse {
    fn from(live: civic::Corpse) -> Self {
        Self {
            agent: live.agent,
            market: live.market,
            x: live.x,
            y: live.y,
            death_tick: live.death_tick,
            status: live.status,
            claim: live.claim,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyContract {
    pub worker: u32,
    pub market: u32,
    pub kind: LegacyJobKind,
    pub corpse: u32,
    pub graveyard: u32,
    pub reward: u64,
    pub phase: u8,
    pub timer: f32,
}
impl From<LegacyContract> for civic::Contract {
    fn from(old: LegacyContract) -> Self {
        Self {
            worker: old.worker,
            market: old.market,
            kind: old.kind.into(),
            corpse: old.corpse,
            graveyard: old.graveyard,
            reward: old.reward,
            phase: old.phase,
            timer: old.timer,
        }
    }
}
#[cfg(test)]
impl From<civic::Contract> for LegacyContract {
    fn from(live: civic::Contract) -> Self {
        Self {
            worker: live.worker,
            market: live.market,
            kind: live.kind.into(),
            corpse: live.corpse,
            graveyard: live.graveyard,
            reward: live.reward,
            phase: live.phase,
            timer: live.timer,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyGraveyard {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub progress: f32,
    pub required: f32,
    pub capacity: u32,
    pub buried: Vec<u32>,
    pub ready: bool,
}
impl From<LegacyGraveyard> for civic::Graveyard {
    fn from(old: LegacyGraveyard) -> Self {
        Self {
            market: old.market,
            x: old.x,
            y: old.y,
            progress: old.progress,
            required: old.required,
            capacity: old.capacity,
            buried: old.buried,
            ready: old.ready,
        }
    }
}
#[cfg(test)]
impl From<civic::Graveyard> for LegacyGraveyard {
    fn from(live: civic::Graveyard) -> Self {
        Self {
            market: live.market,
            x: live.x,
            y: live.y,
            progress: live.progress,
            required: live.required,
            capacity: live.capacity,
            buried: live.buried,
            ready: live.ready,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyRelationships {
    pub edges: BTreeMap<(u32, u32), LegacyLink>,
    pub recent: BTreeMap<(u32, u32), u64>,
    pub rng: LegacyRng,
    pub initialized: bool,
    pub interactions: u64,
    pub positive: u64,
    pub negative: u64,
    pub drift_steps: u64,
    pub drift_checks: u64,
    pub capacity_skips: u64,
}
impl From<LegacyRelationships> for relationships::Relationships {
    fn from(old: LegacyRelationships) -> Self {
        Self {
            edges: old.edges.into_iter().map(|(k, v)| (k, v.into())).collect(),
            recent: old.recent,
            rng: old.rng.into(),
            initialized: old.initialized,
            interactions: old.interactions,
            positive: old.positive,
            negative: old.negative,
            drift_steps: old.drift_steps,
            drift_checks: old.drift_checks,
            capacity_skips: old.capacity_skips,
            neighbours: Default::default(),
            due: Default::default(),
        }
    }
}
#[cfg(test)]
impl From<relationships::Relationships> for LegacyRelationships {
    fn from(live: relationships::Relationships) -> Self {
        Self {
            edges: live.edges.into_iter().map(|(k, v)| (k, v.into())).collect(),
            recent: live.recent,
            rng: live.rng.into(),
            initialized: live.initialized,
            interactions: live.interactions,
            positive: live.positive,
            negative: live.negative,
            drift_steps: live.drift_steps,
            drift_checks: live.drift_checks,
            capacity_skips: live.capacity_skips,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyLink {
    pub score: i16,
    pub next_decay: u64,
    pub last_contact: u64,
    pub contexts: u8,
}
impl From<LegacyLink> for relationships::Link {
    fn from(old: LegacyLink) -> Self {
        Self {
            score: old.score,
            next_decay: old.next_decay,
            last_contact: old.last_contact,
            contexts: old.contexts,
        }
    }
}
#[cfg(test)]
impl From<relationships::Link> for LegacyLink {
    fn from(live: relationships::Link) -> Self {
        Self {
            score: live.score,
            next_decay: live.next_decay,
            last_contact: live.last_contact,
            contexts: live.contexts,
        }
    }
}

// Batch 4: neat-core brain types, Action, and the market lots v4/v5 embed.
// Old saves store an Action as its declaration index, so LegacyAction keeps
// the order (and repr values) it had when this freeze was taken; live
// Action can now be reordered or extended without touching old saves.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum LegacyAction {
    Rest = 0,
    Gather = 1,
    Deliver = 2,
    BuyFood = 3,
    Build = 4,
    Trade = 5,
    Mint = 6,
    Barter = 7,
    WorkForFood = 8,
    MakeCamp = 9,
    Wait = 10,
    BuildGraveyard = 11,
    Gravedigger = 12,
    Court = 13,
    CareFamily = 14,
    CraftTool = 15,
    BuyTool = 16,
    Farm = 17,
    HarvestRice = 18,
    EquipTool = 19,
    ForageMeal = 20,
    StoreCargo = 21,
    Socialize = 22,
    BuildTavern = 23,
    BuildSchool = 24,
    Teach = 25,
    AttendSchool = 26,
    UrbanWork = 27,
    LeaseBusiness = 28,
    OperateBusiness = 29,
    OrderGoods = 30,
    BuyHome = 31,
    RentHome = 32,
    ShopGoods = 33,
    Hunt = 34,
    CollectEstate = 35,
    BuildBank = 36,
    BorrowBusiness = 37,
    RepayLoan = 38,
    HuntWildlife = 39,
    EatCarcass = 40,
    CraftSpear = 41,
    CraftBow = 58,
    Recover = 59,
    Play = 60,
    CollectMeat = 61,
    SellMeat = 62,
    BuyMeat = 63,
    BuyInformation = 64,
    SellInformation = 65,
    NominateSelf = 66,
    DiscussCamp = 67,
    MigrateToCamp = 68,
    DeliverWaterReport = 69,
}
same_variants!(both LegacyAction, Action, [
    Rest, Gather, Deliver, BuyFood, Build, Trade, Mint, Barter, WorkForFood, MakeCamp, Wait, BuildGraveyard, Gravedigger, Court, CareFamily, CraftTool, BuyTool, Farm, HarvestRice, EquipTool, ForageMeal, StoreCargo, Socialize, BuildTavern, BuildSchool, Teach, AttendSchool, UrbanWork, LeaseBusiness, OperateBusiness, OrderGoods, BuyHome, RentHome, ShopGoods, Hunt, CollectEstate, BuildBank, BorrowBusiness, RepayLoan, HuntWildlife, EatCarcass, CraftSpear, CraftBow, Recover, Play, CollectMeat, SellMeat, BuyMeat, BuyInformation, SellInformation, NominateSelf, DiscussCamp, MigrateToCamp, DeliverWaterReport,
]);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LegacyKind {
    Input,
    Tanh,
    Identity,
    Output,
}
same_variants!(LegacyKind, neat_core::Kind, [Input, Tanh, Identity, Output]);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyNode {
    pub id: u32,
    pub kind: LegacyKind,
    pub bias: f32,
}
twin!(LegacyNode, neat_core::Node, [id, kind, bias]);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyGene {
    pub innovation: u64,
    pub from: u32,
    pub to: u32,
    pub weight: f32,
    pub enabled: bool,
}
twin!(
    LegacyGene,
    neat_core::Gene,
    [innovation, from, to, weight, enabled]
);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyGenome {
    pub generation: u32,
    pub nodes: Vec<LegacyNode>,
    pub genes: Vec<LegacyGene>,
}
impl From<LegacyGenome> for Genome {
    fn from(old: LegacyGenome) -> Self {
        Self {
            generation: old.generation,
            nodes: old.nodes.into_iter().map(Into::into).collect(),
            genes: old.genes.into_iter().map(Into::into).collect(),
        }
    }
}
#[cfg(test)]
impl From<Genome> for LegacyGenome {
    fn from(live: Genome) -> Self {
        Self {
            generation: live.generation,
            nodes: live.nodes.into_iter().map(Into::into).collect(),
            genes: live.genes.into_iter().map(Into::into).collect(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyInnovations {
    pub next_node: u32,
    pub next_gene: u64,
    pub edges: BTreeMap<(u32, u32), u64>,
    pub splits: BTreeMap<u64, u32>,
}
twin!(
    LegacyInnovations,
    Innovations,
    [next_node, next_gene, edges, splits]
);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyRng(pub u64);
impl From<LegacyRng> for Rng {
    fn from(old: LegacyRng) -> Self {
        Self(old.0)
    }
}
#[cfg(test)]
impl From<Rng> for LegacyRng {
    fn from(live: Rng) -> Self {
        Self(live.0)
    }
}
// legacy_v13 already froze Lot, Quote and Project with the layout v4 and v5
// wrote; those readers reuse its twins.
impl From<legacy_v13::LegacyLot> for Lot {
    fn from(old: legacy_v13::LegacyLot) -> Self {
        Self {
            quantity: old.quantity,
            origin: old.origin,
        }
    }
}
impl From<legacy_v13::LegacyQuote> for Quote {
    fn from(old: legacy_v13::LegacyQuote) -> Self {
        Self {
            bid: old.bid,
            remaining: old.remaining,
            escrow: old.escrow,
            ask: old.ask,
            offered: old.offered,
        }
    }
}
impl From<legacy_v13::LegacyProject> for Project {
    fn from(old: legacy_v13::LegacyProject) -> Self {
        Self {
            kind: old.kind,
            progress: old.progress,
            required: old.required,
            destination: old.destination,
        }
    }
}
