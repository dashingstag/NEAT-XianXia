use serde::Deserialize;
use serde_json::{json, Value};
use sim_core::{World, DT};
use std::{
    collections::VecDeque,
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Options {
    mode: String,
    agents: usize,
    steps: u32,
    seed: u64,
    threads: usize,
    port: u16,
    out: Option<PathBuf>,
    checkpoint: PathBuf,
    load: bool,
    serial: bool,
    hill: sim_core::Hill,
    seeds: u32,
    map: sim_core::MapSize,
}
fn options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let mut o = Options {
        mode: args.next().unwrap_or("serve".into()),
        agents: 10_000,
        steps: 600,
        seed: 42,
        threads: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(8),
        port: 0,
        out: None,
        checkpoint: PathBuf::from("runs/viewer.snapshot"),
        load: false,
        serial: false,
        hill: Default::default(),
        seeds: 3,
        map: sim_core::MapSize::Classic,
    };
    while let Some(k) = args.next() {
        if k == "--load" {
            o.load = true;
            continue;
        }
        if k == "--serial" {
            o.serial = true;
            continue;
        }
        let v = args.next().ok_or(format!("Missing value for {k}"))?;
        match k.as_str() {
            "--hill-file" => {
                o.hill = serde_json::from_slice(&fs::read(v).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?
            }
            "--agents" => o.agents = v.parse().map_err(|_| "Invalid population")?,
            "--steps" => o.steps = v.parse().map_err(|_| "Invalid steps")?,
            "--seed" => o.seed = v.parse().map_err(|_| "Invalid seed")?,
            "--threads" => o.threads = v.parse().map_err(|_| "Invalid threads")?,
            "--port" => o.port = v.parse().map_err(|_| "Invalid port")?,
            "--out" => o.out = Some(v.into()),
            "--checkpoint" => o.checkpoint = v.into(),
            "--seeds" => o.seeds = v.parse().map_err(|_| "Invalid seeds")?,
            "--map" => {
                o.map = match v.as_str() {
                    "classic" => sim_core::MapSize::Classic,
                    "x4" => sim_core::MapSize::X4,
                    "x25" => sim_core::MapSize::X25,
                    "x100" => sim_core::MapSize::X100,
                    _ => return Err("--map must be classic|x4|x25|x100".into()),
                }
            }
            _ => return Err(format!("Unknown option {k}")),
        }
    }
    if ![
        "serve",
        "bench",
        "verify",
        "civic-demo",
        "life-demo",
        "school-demo",
        "urban-demo",
        "soak",
    ]
    .contains(&o.mode.as_str())
    {
        return Err("Usage: sim-runner serve|bench|verify [--agents 10000] [--steps 600] [--seed 42] [--threads 8] [--out runs/report.json] [--load] [--checkpoint runs/viewer.snapshot] [--serial]".into());
    }
    if matches!(
        o.mode.as_str(),
        "civic-demo" | "life-demo" | "school-demo" | "urban-demo"
    ) && o.checkpoint == PathBuf::from("runs/viewer.snapshot")
    {
        o.checkpoint = PathBuf::from(if o.mode == "urban-demo" {
            "runs/urban-demo.snapshot"
        } else if o.mode == "school-demo" {
            "runs/school-demo.snapshot"
        } else if o.mode == "life-demo" {
            "runs/life-demo.snapshot"
        } else {
            "runs/civic-demo.snapshot"
        });
    }
    let step_limit = if o.mode == "soak" {
        6_000_000
    } else {
        1_000_000
    };
    if o.threads == 0 || o.threads > 256 || o.steps > step_limit {
        return Err("Invalid thread/step budget".into());
    }
    Ok(o)
}
fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let o = options()?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(o.threads)
        .build_global()
        .map_err(|e| e.to_string())?;
    let initialized = Instant::now();
    let mut w = if o.load {
        World::load(&o.checkpoint)?
    } else {
        World::new_with_size(o.agents, o.seed, o.hill, o.map)?
    };
    if o.mode == "civic-demo" {
        for _ in 0..300 {
            w.step(!o.serial);
        }
        w.mortality_test(0, 12)?;
    }
    if o.mode == "urban-demo" {
        w.prepare_urban_demo()?;
    }
    if o.mode == "school-demo" {
        w.prepare_school_demo()?;
    }
    if o.mode == "life-demo" {
        w.prepare_family_demo()?;
    }
    let initialize_seconds = initialized.elapsed().as_secs_f64();
    if o.mode == "serve" {
        return serve(w, &o);
    }
    if o.mode == "soak" {
        return soak(&o);
    }
    let start_decisions = w.stats.decisions;
    let started = Instant::now();
    let mut times = Vec::new();
    for tick in 0..o.steps {
        let start = Instant::now();
        w.step(!o.serial);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        if tick % 100 == 0 || o.mode == "verify" {
            w.check()?;
        }
    }
    let seconds = started.elapsed().as_secs_f64();
    w.check()?;
    times.sort_by(|a, b| a.total_cmp(b));
    let percentile = |q: f64| {
        times
            .get(((times.len().saturating_sub(1)) as f64 * q) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let max_age = w
        .agents
        .last_decision
        .iter()
        .enumerate()
        .filter(|(i, _)| w.agents.alive[*i])
        .map(|(_, t)| w.tick.saturating_sub(*t))
        .max()
        .unwrap_or(0);
    let mut report = json!({"implementation":"native Rust / packed agents / Rayon / compiled feed-forward NEAT","mode":o.mode,"serial":o.serial,"threads":o.threads,"steps":o.steps,"initialize_seconds":initialize_seconds,"step_wall_seconds":seconds,"simulated_seconds":o.steps as f32*DT,"simulated_seconds_per_wall_second":o.steps as f64*DT as f64/seconds.max(1e-9),"scheduled_decisions":w.stats.decisions-start_decisions,"decisions_per_wall_second":(w.stats.decisions-start_decisions)as f64/seconds.max(1e-9),"p50_tick_ms":percentile(0.5),"p95_tick_ms":percentile(0.95),"p99_tick_ms":percentile(0.99),"max_living_decision_age_ticks":max_age,"node_mean":w.genomes.iter().map(|g|g.nodes.len()).sum::<usize>()as f64/w.genomes.len()as f64,"gene_mean":w.genomes.iter().map(|g|g.genes.len()).sum::<usize>()as f64/w.genomes.len()as f64,"accounting_ok":true,"world":w.summary()});
    if o.mode == "urban-demo" {
        report["business_candidates"] =
            json!((0..16).filter_map(|i| w.trace(i)).collect::<Vec<_>>());
    }
    if let Some(path) = &o.out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).map_err(|e| e.to_string())?;
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if o.mode == "verify"
        || o.mode == "civic-demo"
        || o.mode == "life-demo"
        || o.mode == "school-demo"
        || o.mode == "urban-demo"
    {
        if let Some(parent) = o.checkpoint.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        w.save(&o.checkpoint)?;
        let mut restored = World::load(&o.checkpoint)?;
        let mut reference = w.clone();
        for _ in 0..20 {
            reference.step(false);
            restored.step(true);
        }
        if reference.agents != restored.agents || reference.markets != restored.markets {
            return Err("Replay mismatch".into());
        }
        println!("Checkpoint replay: exact match after 20 additional ticks.");
    }
    Ok(())
}

/// Headless multi-seed soak: runs each seed for --steps ticks and reports
/// population dynamics, trade, settlement foundings and ecology. The world
/// digest lets CI detect unintended nondeterminism between revisions.
fn soak(o: &Options) -> Result<(), String> {
    if o.steps == 0 {
        return Err("Soak requires at least one step".into());
    }
    if o.seeds == 0 || o.seeds > 64 {
        return Err("Seeds must be 1..64".into());
    }
    let mut runs = Vec::new();
    for s in 0..o.seeds {
        let seed = o.seed.wrapping_add(s as u64);
        let mut w = World::new_with_size(o.agents, seed, o.hill, o.map)?;
        let initial_species = w.living_wildlife_by_species();
        let mut extinct_species = std::collections::BTreeMap::new();
        let mut times = Vec::with_capacity(o.steps as usize);
        for tick in 0..o.steps {
            let start = Instant::now();
            w.step(!o.serial);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            if (tick + 1) % 300_000 == 0 {
                eprintln!(
                    "seed {seed}, day {}: {:?}",
                    (tick + 1) / 3000,
                    w.living_wildlife_by_species()
                );
            }
            if tick % 100 == 0 || tick + 1 == o.steps {
                w.check()?;
                for (species, count) in w.living_wildlife_by_species() {
                    if count == 0 && initial_species[species] > 0 {
                        extinct_species.entry(species).or_insert(w.tick);
                    }
                }
            }
        }
        w.check()?;
        times.sort_by(|a, b| a.total_cmp(b));
        let digest = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            w.checkpoint_payload()?.hash(&mut h);
            format!("{:016x}", h.finish())
        };
        let animals = w.animal_count();
        let wildlife_deaths = w.wildlife.ledger.deaths;
        let wildlife_births = w.wildlife.ledger.births;
        let towns = w.markets.iter().filter(|m| m.village).count();
        runs.push(json!({
            "seed": seed,
            "tick": w.tick,
            "living": w.living(),
            "births": w.stats.births,
            "deaths": w.stats.deaths,
            "trades": w.stats.trades,
            "camps_founded": w.stats.camps_founded,
            "markets": w.markets.len(),
            "villages": towns,
            "animals": animals,
            "initial_living_by_species": initial_species,
            "living_by_species": w.living_wildlife_by_species(),
            "hunting": w.hunting_stats(),
            "first_observed_extinction_tick": extinct_species,
            "wildlife_births": wildlife_births,
            "wildlife_deaths": wildlife_deaths,
            "p50_tick_ms": times[(times.len() - 1) / 2],
            "p95_tick_ms": times[((times.len() - 1) as f64 * 0.95) as usize],
            "world_digest": digest,
        }));
    }
    let report =
        json!({"mode":"soak","agents":o.agents,"steps":o.steps,"seeds":o.seeds,"runs":runs});
    if let Some(path) = &o.out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).map_err(|e| e.to_string())?;
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
    StreamBuildings,
    ForgetFootpaths {
        segment: Option<[f32; 4]>,
    },
    WatchWealth {
        enabled: bool,
    },
    Pause {
        value: bool,
    },
    SetPersona {
        market: usize,
        profile: u8,
    },
    TimeOfDay {
        hour: f32,
    },
    MortalityTest {
        market: usize,
        count: usize,
    },
    Speed {
        value: f32,
    },
    Step {
        ticks: u32,
    },
    AdvanceDays {
        days: u32,
    },
    Select {
        agent: usize,
    },
    SelectAnimal {
        animal: usize,
    },
    AddResource {
        kind: usize,
        x: f32,
        y: f32,
        quantity: u64,
    },
    Reset {
        agents: usize,
        seed: u64,
    },
    Save,
    Load,
    MapLayers {
        #[serde(default = "yes")]
        terrain: bool,
    },
}
fn yes() -> bool {
    true
}
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let n = group
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= group.len() {
                ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}
/// Terrain raster and fog-of-war mask for viewers. Sent on request, not in
/// every snapshot, because the raster only changes with the map.
fn map_layers(world: &mut World, terrain: bool) -> Value {
    let (cols, rows, known) = world.explored.known_mask(&world.map.spec);
    let mut layers = json!({
        "map": world.map.spec,
        "explored": {"cols": cols, "rows": rows, "chunk": sim_core::CHUNK_SIZE, "known": base64(&known)},
    });
    if terrain {
        let r = world.map.raster();
        let rivers: Vec<Value> = world
            .map
            .rivers()
            .iter()
            .map(|river| json!({"w": river.width, "p": river.points}))
            .collect();
        let labels: Vec<Value> = r
            .region_labels
            .iter()
            .map(|(n, x, y)| json!({"n": n, "x": x, "y": y}))
            .collect();
        layers["terrain"] = json!({
            "cols": r.cols, "rows": r.rows, "cell": r.cell,
            "classes": base64(&r.terrain), "elevation": base64(&r.elevation), "qi": base64(&r.qi),
            "regions": base64(&r.regions), "region_labels": labels, "rivers": rivers,
        });
    }
    layers
}
fn messages(buffer: &mut Vec<u8>) -> Result<Vec<Value>, String> {
    let mut messages = vec![];
    loop {
        if buffer.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes(buffer[..4].try_into().unwrap()) as usize;
        if len == 0 || len > 1_048_576 {
            return Err("Invalid command frame size".into());
        }
        if buffer.len() < 4 + len {
            break;
        }
        messages.push(serde_json::from_slice(&buffer[4..4 + len]).map_err(|e| e.to_string())?);
        buffer.drain(..4 + len);
        if messages.len() >= 32 {
            break;
        }
    }
    Ok(messages)
}
const MAX_VIEWER_FRAME: usize = 4 * 1024 * 1024;

fn frame(payload: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(payload);
    out
}
struct Client {
    stream: TcpStream,
    input: Vec<u8>,
    output: Vec<u8>,
    offset: usize,
    replies: VecDeque<Vec<u8>>,
    stream_buildings: bool,
    buildings: Vec<Value>,
    building_updates: VecDeque<Vec<u8>>,
    next_buildings: Instant,
    oversize_reported: bool,
}
impl Client {
    fn new(stream: TcpStream) -> Result<Self, String> {
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            input: vec![],
            output: vec![],
            offset: 0,
            replies: VecDeque::new(),
            stream_buildings: false,
            buildings: Vec::new(),
            building_updates: VecDeque::new(),
            next_buildings: Instant::now(),
            oversize_reported: false,
        })
    }
    fn read(&mut self) -> Result<Vec<Value>, String> {
        let mut block = [0u8; 8192];
        loop {
            match self.stream.read(&mut block) {
                Ok(0) => return Err("Viewer disconnected".into()),
                Ok(n) => {
                    self.input.extend(&block[..n]);
                    if self.input.len() > 2_097_152 {
                        return Err("Input budget exceeded".into());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.to_string()),
            }
        }
        messages(&mut self.input)
    }
    fn reply(&mut self, text: &str, ok: bool) {
        self.reply_value(json!({"ok":ok,"message":text}));
    }
    fn reply_value(&mut self, value: Value) {
        let mut bytes = b"XNC1".to_vec();
        bytes.extend(serde_json::to_vec(&value).unwrap());
        if bytes.len() > MAX_VIEWER_FRAME {
            bytes = b"XNC1".to_vec();
            bytes.extend_from_slice(br#"{"ok":false,"message":"Viewer reply exceeds the 4 MiB limit."}"#);
        }
        if self.replies.len() >= 8 {
            self.replies.pop_front();
        }
        self.replies.push_back(frame(bytes));
    }
    fn reset_buildings(&mut self) {
        if self.stream_buildings {
            self.buildings.clear();
            self.building_updates.clear();
            self.building_updates.push_back(frame(br#"XNC1{"building_count":0}"#.to_vec()));
            self.next_buildings = Instant::now();
        }
    }

    fn update_buildings(&mut self, world: &World, now: Instant) {
        if !self.stream_buildings || now < self.next_buildings || !self.building_updates.is_empty() {
            return;
        }
        self.next_buildings = now + Duration::from_secs(1);
        let rows = world.viewer_buildings();
        if rows == self.buildings {
            return;
        }
        for (chunk, batch) in rows.chunks(256).enumerate() {
            let start = chunk * 256;
            if self.buildings.get(start..start + batch.len()) != Some(batch) {
                let mut bytes = b"XNC1".to_vec();
                bytes.extend(serde_json::to_vec(&json!({"building_batch":batch,"building_offset":start})).unwrap());
                self.building_updates.push_back(frame(bytes));
            }
        }
        let mut commit = b"XNC1".to_vec();
        commit.extend(serde_json::to_vec(&json!({"building_count":rows.len()})).unwrap());
        self.building_updates.push_back(frame(commit));
        self.buildings = rows;
    }

    fn send(&mut self, snapshot: Option<Vec<u8>>) -> Result<(), String> {
        let snapshot = snapshot.filter(|bytes| {
            if bytes.len() <= MAX_VIEWER_FRAME {
                self.oversize_reported = false;
                true
            } else {
                if !self.oversize_reported {
                    self.reply("World snapshot exceeds the 4 MiB viewer limit.", false);
                    self.oversize_reported = true;
                }
                false
            }
        });
        if self.offset == self.output.len() {
            self.output = if let Some(reply) = self.replies.pop_front() {
                reply
            } else if let Some(update) = self.building_updates.pop_front() {
                update
            } else if let Some(bytes) = snapshot {
                frame(bytes)
            } else {
                vec![]
            };
            self.offset = 0;
        }
        while self.offset < self.output.len() {
            match self.stream.write(&self.output[self.offset..]) {
                Ok(0) => return Err("Viewer write closed".into()),
                Ok(n) => self.offset += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
}
fn serve(mut world: World, o: &Options) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", o.port)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut client: Option<Client> = None;
    let mut paused = true;
    let mut speed = 1.0f32;
    let mut advance_remaining = 0u64;
    let mut wealth = sim_core::WealthView::default();
    let mut selected = Some(0usize);
    let mut selected_animal = None;
    let mut last = Instant::now();
    let mut next_snapshot = Instant::now();
    let mut accumulator = 0.0f32;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if let Some(path) = env::var_os("XIANXIA_READY_FILE") {
        let path = PathBuf::from(path);
        let temp = path.with_extension("tmp");
        let ready = json!({"pid":std::process::id(),"port":port});
        fs::write(&temp, ready.to_string()).map_err(|e| e.to_string())?;
        fs::rename(temp, path).map_err(|e| e.to_string())?;
    }
    println!("Native simulation ready on 127.0.0.1:{port}; {} individual brains; {} CPU workers. Waiting for viewer. Starts paused.",world.agents.len(),o.threads);
    loop {
        if client.is_none() {
            if let Ok((stream, _)) = listener.accept() {
                wealth = Default::default();
                client = Some(Client::new(stream)?);
                eprintln!("Viewer connected");
            }
        }
        let incoming = if let Some(c) = client.as_mut() {
            match c.read() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    client = None;
                    advance_remaining = 0;
                    paused = true;
                    vec![]
                }
            }
        } else {
            vec![]
        };
        for value in incoming {
            let mut layers = None;
            let result: Result<String, String> =
                (|| match serde_json::from_value::<Command>(value).map_err(|e| e.to_string())? {
                    Command::StreamBuildings => {
                        if let Some(c) = client.as_mut() {
                            c.stream_buildings = true;
                        }
                        Ok("Building stream enabled".into())
                    }
                    Command::WatchWealth { enabled } => {
                        wealth.subscribe(enabled);
                        Ok("Wealth subscription updated".into())
                    }
                    Command::MortalityTest { market, count } => {
                        wealth.invalidate();
                        let n = world.mortality_test(market, count)?;
                        Ok(format!(
                            "Test: {n} nearby individuals died; run the simulation to observe jobs"
                        ))
                    }
                    Command::TimeOfDay { hour } => {
                        world.set_hour(hour)?;
                        Ok(format!("Clock set to {hour:.1}:00"))
                    }
                    Command::AdvanceDays { days } => {
                        if days == 0 || days > 30 {
                            return Err("Advance 1..30 days at a time".into());
                        }
                        advance_remaining = days as u64 * 3000;
                        paused = false;
                        accumulator = 0.0;
                        Ok(format!("Advancing {days} days; Pause cancels"))
                    }
                    Command::Pause { value } => {
                        if value {
                            advance_remaining = 0;
                        }
                        paused = value;
                        Ok(if paused { "Paused" } else { "Running" }.into())
                    }
                    Command::Speed { value } => {
                        if !value.is_finite() || !(0.1..=16.0).contains(&value) {
                            return Err("Speed must be 0.1..16".into());
                        }
                        speed = value;
                        Ok(format!("Speed {value}x"))
                    }
                    Command::Step { ticks } => {
                        if ticks == 0 || ticks > 10_000 {
                            return Err("Step budget must be 1..10,000".into());
                        }
                        for _ in 0..ticks {
                            world.watch_person(selected);
                            world.step(true);
                        }
                        world.check()?;
                        Ok(format!("Advanced {ticks} ticks; accounts balanced"))
                    }
                    Command::SetPersona { market, profile } => {
                        world.set_village_persona(market, profile)?;
                        Ok("Village profile updated".into())
                    }
                    Command::AddResource {
                        kind,
                        x,
                        y,
                        quantity,
                    } => {
                        let id = world.add_resource(kind, x, y, quantity)?;
                        Ok(format!("Added resource site #{id}"))
                    }
                    Command::SelectAnimal { animal } => {
                        if animal >= world.animal_count() {
                            return Err("No such animal".into());
                        }
                        selected_animal = Some(animal);
                        Ok(format!("Inspecting animal {animal}"))
                    }
                    Command::Select { agent } => {
                        if agent >= world.agents.len() {
                            return Err("No such agent".into());
                        }
                        selected = Some(agent);
                        selected_animal = None;
                        Ok(format!("Inspecting agent {agent}"))
                    }
                    Command::Reset { agents, seed } => {
                        advance_remaining = 0;
                        wealth.invalidate();
                        world = World::new_with_size(agents, seed, world.hill, world.map.spec.size)?;
                        if let Some(c) = client.as_mut() {
                            c.reset_buildings();
                        }
                        selected = Some(0);
                        selected_animal = None;
                        paused = true;
                        accumulator = 0.0;
                        Ok("New native world created".into())
                    }
                    Command::Save => {
                        if let Some(parent) = o.checkpoint.parent() {
                            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                        }
                        world.check()?;
                        world.save(&o.checkpoint)?;
                        Ok(format!("Saved {}", o.checkpoint.display()))
                    }
                    Command::Load => {
                        advance_remaining = 0;
                        wealth.invalidate();
                        world = World::load(&o.checkpoint)?;
                        if let Some(c) = client.as_mut() {
                            c.reset_buildings();
                        }
                        paused = true;
                        selected = Some(0);
                        selected_animal = None;
                        Ok("Restored checkpoint and rebuilt compiled brains".into())
                    }
                    Command::ForgetFootpaths { segment } => {
                        let count = world.forget_cached_footpaths(segment);
                        Ok(format!("Removed {count} cached route entries"))
                    }
                    Command::MapLayers { terrain } => {
                        layers = Some(map_layers(&mut world, terrain));
                        Ok("Map layers sent".into())
                    }
                })();
            if let Some(c) = client.as_mut() {
                match (result, layers) {
                    (Ok(text), Some(map_layers)) => {
                        c.reply_value(json!({"ok":true,"message":text,"map_layers":map_layers}))
                    }
                    (Ok(text), None) => c.reply(&text, true),
                    (Err(text), _) => c.reply(&text, false),
                }
            }
        }
        let now = Instant::now();
        let elapsed = now.duration_since(last).as_secs_f32().min(0.25);
        last = now;
        if !paused && client.is_some() && advance_remaining == 0 {
            accumulator += elapsed * speed;
        } else {
            accumulator = 0.0;
        }
        let mut budget = 32;
        while accumulator >= DT && budget > 0 {
            world.watch_person(selected);
            world.step(true);
            accumulator -= DT;
            budget -= 1;
        }
        if !paused && client.is_some() && advance_remaining > 0 {
            let mut batch = 32;
            while batch > 0 && advance_remaining > 0 {
                world.watch_person(selected);
                world.step(true);
                advance_remaining -= 1;
                batch -= 1;
            }
            if advance_remaining == 0 {
                paused = true;
                let result = world.check();
                if let Some(c) = client.as_mut() {
                    match result {
                        Ok(()) => c.reply("Day advance complete; accounts balanced", true),
                        Err(e) => c.reply(&e, false),
                    }
                }
            }
        }
        let snapshot = if now >= next_snapshot && client.is_some() {
            next_snapshot = now + Duration::from_millis(100);
            Some(world.viewer_snapshot_with_buildings(
                20_000,
                selected,
                paused,
                selected_animal,
                wealth.report(&world),
                !client.as_ref().is_some_and(|c| c.stream_buildings),
            ))
        } else {
            None
        };
        if let Some(c) = client.as_mut() {
            c.update_buildings(&world, now);
            if let Err(e) = c.send(snapshot) {
                eprintln!("{e}");
                client = None;
                advance_remaining = 0;
                paused = true;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_command_frames_are_reassembled() {
        let body = br#"{"command":"pause","value":true}"#;
        let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
        bytes.extend(&body[..5]);
        assert!(messages(&mut bytes).unwrap().is_empty());
        bytes.extend(&body[5..]);
        assert_eq!(messages(&mut bytes).unwrap()[0]["command"], "pause");
        assert!(bytes.is_empty());
    }
    #[test]
    fn oversized_commands_are_rejected() {
        assert!(messages(&mut 2_000_000u32.to_le_bytes().to_vec()).is_err());
    }
}
