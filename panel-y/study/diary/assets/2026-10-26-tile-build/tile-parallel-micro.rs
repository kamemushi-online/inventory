use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct Tile { start: usize, end: usize, min: f32, max: f32, finite: bool }
struct Channel { index: usize, values: Vec<f32> }
struct Cache { index: usize, tiles: Vec<Tile>, elapsed: Duration }

// Same finite min/max loop and tile layout as build_line_tile_cache.
fn build(channel: &Channel) -> Cache {
    let started = Instant::now();
    let values = black_box(&channel.values);
    let tile_size = 256;
    let mut tiles = Vec::with_capacity(values.len().div_ceil(tile_size));
    let mut start = 0;
    while start < values.len() {
        let end = (start + tile_size).min(values.len());
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for value in &values[start..end] {
            if value.is_finite() {
                min = min.min(*value);
                max = max.max(*value);
            }
        }
        let finite = min.is_finite() && max.is_finite();
        tiles.push(Tile { start, end, min, max, finite });
        start = end;
    }
    Cache { index: channel.index, tiles, elapsed: started.elapsed() }
}

fn batch(channels: &[&Channel], workers: usize) -> Vec<Cache> {
    if workers == 1 { return channels.iter().map(|ch| build(ch)).collect(); }
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers - 1);
        for worker in 1..workers {
            handles.push(scope.spawn(move || {
                channels.iter().skip(worker).step_by(workers).map(|ch| build(ch)).collect::<Vec<_>>()
            }));
        }
        let mut results: Vec<_> = channels.iter().step_by(workers).map(|ch| build(ch)).collect();
        for handle in handles { results.extend(handle.join().unwrap()); }
        results.sort_unstable_by_key(|cache| cache.index);
        results
    })
}

fn mix(state: &mut u64, bits: u64) { *state = (*state ^ bits).wrapping_mul(0x100000001b3); }
fn digest(caches: &[Cache]) -> u64 {
    let mut state = 0xcbf29ce484222325;
    for cache in caches {
        mix(&mut state, cache.index as u64);
        for tile in &cache.tiles {
            mix(&mut state, tile.start as u64); mix(&mut state, tile.end as u64);
            mix(&mut state, u64::from(tile.min.to_bits())); mix(&mut state, u64::from(tile.max.to_bits()));
            mix(&mut state, u64::from(tile.finite));
        }
    }
    state
}

fn run(lines: &[&Channel], steps: &[&Channel], workers: usize) -> (Vec<Cache>, f64, f64, f64, f64) {
    let start = Instant::now();
    let mut caches = batch(lines, workers);
    let line_ms = start.elapsed().as_secs_f64() * 1000.;
    let step_start = Instant::now();
    for ch in steps { caches.push(build(ch)); }
    let step_ms = step_start.elapsed().as_secs_f64() * 1000.;
    let total_ms = start.elapsed().as_secs_f64() * 1000.;
    let sum_ms = caches.iter().map(|cache| cache.elapsed.as_secs_f64() * 1000.).sum();
    (caches, line_ms, step_ms, total_ms, sum_ms)
}

fn main() {
    let init = Instant::now();
    let mut channels = Vec::with_capacity(36);
    for n in [10_000_000, 8_000_000, 12_000_000, 6_000_000] {
        for channel in 0..9 {
            let values = (0..n).map(|index| {
                if (512..768).contains(&index) && channel == 0 { return f32::NAN; }
                match index % 10_007 {
                    0 => f32::NAN,
                    1 => f32::INFINITY,
                    2 => f32::NEG_INFINITY,
                    3 => f32::from_bits(1),
                    4 => -0.0,
                    _ => ((index % 65521) as f32) * 0.125 - 1000. + channel as f32 * 0.5,
                }
            }).collect();
            channels.push(Channel { index: channels.len(), values });
        }
    }
    let init_ms = init.elapsed().as_secs_f64() * 1000.;
    let lines: Vec<_> = channels.iter().filter(|ch| ch.index % 9 != 8).collect();
    let steps: Vec<_> = channels.iter().filter(|ch| ch.index % 9 == 8).collect();
    let (reference, ..) = run(&lines, &steps, 1);
    let expected = digest(&reference);
    let tiles: usize = reference.iter().map(|cache| cache.tiles.len()).sum();
    assert!(reference.iter().any(|cache| cache.tiles.iter().any(|tile| !tile.finite)));
    drop(reference);
    println!("{{\"architecture\":\"{}\",\"logical_cpus\":{},\"pid\":{},\"MallocLargeCache\":\"{}\",\"channels\":36,\"line_channels\":32,\"serial_step_channels\":4,\"samples_per_source\":[10000000,8000000,12000000,6000000],\"total_f32_values\":324000000,\"raw_bytes\":1296000000,\"tile_width\":256,\"tile_count\":{},\"tile_bytes\":{},\"initialize_ms\":{},\"reference_digest\":\"{:016x}\",\"runs\":[",std::env::consts::ARCH,std::thread::available_parallelism().unwrap().get(),std::process::id(),std::env::var("MallocLargeCache").unwrap_or_default(),tiles,tiles*std::mem::size_of::<Tile>(),init_ms,expected);
    let mut count = 0;
    for (round, order) in [[1,2,4],[4,2,1],[2,1,4]].into_iter().enumerate() {
        for workers in order {
            let (caches, line_ms, step_ms, total_ms, sum_ms) = run(&lines, &steps, workers);
            let actual = black_box(digest(black_box(&caches)));
            assert_eq!(actual, expected);
            if count > 0 { println!(","); }
            print!("{{\"round\":{},\"workers_including_caller\":{},\"line_batch_wall_ms\":{},\"step_serial_wall_ms\":{},\"total_wall_ms\":{},\"sum_channel_build_ms\":{},\"exact_digest_match\":true}}",round+1,workers,line_ms,step_ms,total_ms,sum_ms);
            count += 1;
            drop(caches);
        }
    }
    println!("],\"scope\":\"Synthetic resident arrays; existing scalar tile reducer; 32 Line channels batch-built with bounded scoped threads including caller, followed by 4 serial Step channels. Timings include cache allocations, thread spawn/join, and result assembly; exclude digest and cache disposal. Sum channel durations overlap under parallelism and are not UI wall time.\"}}");
}
