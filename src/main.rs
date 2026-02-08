use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use std::time::Instant;
use tracing::{info, instrument, span, Level};

//using "Copy" traits for raw performance on simple structs
#[derive(Clone, Copy, Debug)]
struct Body {
    id: usize,
    pos: [f64; 3],
    vel: [f64; 3],
    mass: f64,
}

struct Universe {
    bodies: Vec<Body>,
    g_const: f64,
    softening: f64,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Sedaro Optimization Showcase")]
struct Args {
    //bodies to simulate
    #[arg(short, long, default_value_t = 5_000)]
    count: usize,
    #[arg(short, long, value_enum, default_value_t = Mode::Parallel)]
    mode: Mode,

    //number of simulation ticks
    #[arg(short, long, default_value_t = 100)]
    ticks: usize,
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    Serial,
    Parallel,
}

impl Universe {
    //simulating a galactic scenario one center, thousands of orbiters
    fn new(count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let mut bodies = Vec::with_capacity(count);

        //black hole
        bodies.push(Body {
            id: 0,
            pos: [0.0, 0.0, 0.0],
            vel: [0.0, 0.0, 0.0],
            mass: 1_000_000.0,
        });

        //stars
        for i in 1..count {
            use rand::Rng;
            let dist = rng.gen_range(100.0..1000.0);
            let angle = rng.gen_range(0.0..std::f64::consts::PI * 2.0);
            let velocity = (1_000_000.0_f64 / dist).sqrt(); 

            bodies.push(Body {
                id: i,
                pos: [dist * angle.cos(), dist * angle.sin(), 0.0],
                vel: [-velocity * angle.sin(), velocity * angle.cos(), 0.0],
                mass: rng.gen_range(1.0..10.0),
            });
        }

        info!("Universe created with {} bodies.", count);
        Universe {
            bodies,
            g_const: 1.0,
            softening: 1e-5,
        }
    }

    //acceleration on target caused by source
    #[inline(always)]
    fn compute_force(&self, target: &Body, source: &Body) -> [f64; 3] {
        let dx = source.pos[0] - target.pos[0];
        let dy = source.pos[1] - target.pos[1];
        let dz = source.pos[2] - target.pos[2];

        let dist_sq = dx * dx + dy * dy + dz * dz + self.softening;
        let dist = dist_sq.sqrt();
        let f = (self.g_const * source.mass) / dist_sq;

        [f * dx / dist, f * dy / dist, f * dz / dist]
    }

    //rayon parallel iterator
    #[instrument(skip(self), name = "tick_parallel")]
    fn step_parallel(&mut self, dt: f64) {
        let positions: Vec<[f64; 3]> = self.bodies.iter().map(|b| b.pos).collect();
        let masses: Vec<f64> = self.bodies.iter().map(|b| b.mass).collect();

        //computing accelerations in parallel
        let accelerations: Vec<[f64; 3]> = self.bodies
            .par_iter()
            .map(|body| {
                let mut acc = [0.0; 3];
                //iterating over the separated data to avoid borrowing the whole body struct
                for (i, pos) in positions.iter().enumerate() {
                    if body.id != i {
                        let dx = pos[0] - body.pos[0];
                        let dy = pos[1] - body.pos[1];
                        let dz = pos[2] - body.pos[2];

                        let dist_sq = dx * dx + dy * dy + dz * dz + self.softening;
                        let dist = dist_sq.sqrt();
                        let f = (self.g_const * masses[i]) / dist_sq;

                        acc[0] += f * dx / dist;
                        acc[1] += f * dy / dist;
                        acc[2] += f * dz / dist;
                    }
                }
                acc
            })
            .collect();

        self.bodies.par_iter_mut().zip(accelerations.par_iter()).for_each(|(body, acc)| {
            body.vel[0] += acc[0] * dt;
            body.vel[1] += acc[1] * dt;
            body.vel[2] += acc[2] * dt;
            body.pos[0] += body.vel[0] * dt;
            body.pos[1] += body.vel[1] * dt;
            body.pos[2] += body.vel[2] * dt;
        });
    }

    //serial iterator
    #[instrument(skip(self), name = "tick_serial")]
    fn step_serial(&mut self, dt: f64) {
        let updates: Vec<[f64; 3]> = self.bodies
            .iter()
            .map(|body| {
                let mut acc = [0.0; 3];
                for other in &self.bodies {
                    if body.id != other.id {
                        let f = self.compute_force(body, other);
                        acc[0] += f[0];
                        acc[1] += f[1];
                        acc[2] += f[2];
                    }
                }
                acc
            })
            .collect();

        for (i, body) in self.bodies.iter_mut().enumerate() {
            let acc = updates[i];
            body.vel[0] += acc[0] * dt;
            body.vel[1] += acc[1] * dt;
            body.vel[2] += acc[2] * dt;
            body.pos[0] += body.vel[0] * dt;
            body.pos[1] += body.vel[1] * dt;
            body.pos[2] += body.vel[2] * dt;
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    
    info!("Initializing Simulation in {:?} mode...", args.mode);
    let mut universe = Universe::new(args.count);

    let start_time = Instant::now();
    let dt = 0.01;

    for i in 0..args.ticks {
        let _span = span!(Level::INFO, "sim_step", tick = i).entered();
        match args.mode {
            Mode::Parallel => universe.step_parallel(dt),
            Mode::Serial => universe.step_serial(dt),
        }
    }

    let duration = start_time.elapsed();
    let per_tick = duration / args.ticks as u32;

    println!("\n--- RESULTS ---");
    println!("Mode:       {:?}", args.mode);
    println!("Bodies:     {}", args.count);
    println!("Total Time: {:.2?}", duration);
    println!("Avg Tick:   {:.2?}", per_tick);
    println!("----------------\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_determinism() {
        let mut universe_serial = Universe::new(100);
        let mut universe_parallel = Universe::new(100);
        universe_parallel.bodies = universe_serial.bodies.clone();

        let dt = 0.01;
        
        universe_serial.step_serial(dt);
        universe_parallel.step_parallel(dt);

        //for checking race conditions.
        for i in 0..universe_serial.bodies.len() {
            let s_pos = universe_serial.bodies[i].pos;
            let p_pos = universe_parallel.bodies[i].pos;

            let diff_x = (s_pos[0] - p_pos[0]).abs();
            let diff_y = (s_pos[1] - p_pos[1]).abs();
            let diff_z = (s_pos[2] - p_pos[2]).abs();

            assert!(diff_x < 1e-10, "Drift detected in X at index {}", i);
            assert!(diff_y < 1e-10, "Drift detected in Y at index {}", i);
            assert!(diff_z < 1e-10, "Drift detected in Z at index {}", i);
        }
    }
}