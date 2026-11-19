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
            "--agents" => o.agents = v.parse().map_err(|_| "Invalid population")?,
            "--steps" => o.steps = v.parse().map_err(|_| "Invalid steps")?,
            "--seed" => o.seed = v.parse().map_err(|_| "Invalid seed")?,
            "--threads" => o.threads = v.parse().map_err(|_| "Invalid threads")?,
            "--port" => o.port = v.parse().map_err(|_| "Invalid port")?,
            "--out" => o.out = Some(v.into()),
            "--checkpoint" => o.checkpoint = v.into(),
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
    if o.threads == 0 || o.threads > 256 || o.steps > 1_000_000 {
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
        World::new(o.agents, o.seed)?
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

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
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
        let mut bytes = b"XNC1".to_vec();
        bytes.extend(serde_json::to_vec(&json!({"ok":ok,"message":text})).unwrap());
        if self.replies.len() >= 8 {
            self.replies.pop_front();
        }
        self.replies.push_back(frame(bytes));
    }
    fn send(&mut self, snapshot: Option<Vec<u8>>) -> Result<(), String> {
        if self.offset == self.output.len() {
            self.output = if let Some(reply) = self.replies.pop_front() {
                reply
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
            let result: Result<String, String> =
                (|| match serde_json::from_value::<Command>(value).map_err(|e| e.to_string())? {
                    Command::MortalityTest { market, count } => {
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
                        world = World::new(agents, seed)?;
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
                        world = World::load(&o.checkpoint)?;
                        paused = true;
                        selected = Some(0);
                        selected_animal = None;
                        Ok("Restored checkpoint and rebuilt compiled brains".into())
                    }
                })();
            if let Some(c) = client.as_mut() {
                match result {
                    Ok(text) => c.reply(&text, true),
                    Err(text) => c.reply(&text, false),
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
            world.step(true);
            accumulator -= DT;
            budget -= 1;
        }
        if !paused && client.is_some() && advance_remaining > 0 {
            let mut batch = 32;
            while batch > 0 && advance_remaining > 0 {
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
            Some(world.snapshot_with_wildlife(20_000, selected, paused, selected_animal))
        } else {
            None
        };
        if let Some(c) = client.as_mut() {
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
