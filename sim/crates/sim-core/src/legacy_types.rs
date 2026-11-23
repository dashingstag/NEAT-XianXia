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
