use crate::position_parser::{SimulationData, TimePoint};

use glam::Vec3A;
use once_cell::sync::OnceCell;
use plotters::prelude::*;
use rand::{distributions::Alphanumeric, Rng};

use indexmap::IndexMap;
use std::ops::{Deref, Range};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(serde::Serialize, serde::Deserialize)]
struct Parameter {
    name: String,
    #[serde(skip, default = "optim_new")]
    optim: tpe::TpeOptimizer,
}

const PARAM_MAX: f64 = 18.0;
const PARAM_MIN: f64 = 0.0;

fn optim_new() -> tpe::TpeOptimizer {
    tpe::TpeOptimizer::new(
        tpe::parzen_estimator(),
        tpe::range(PARAM_MIN, PARAM_MAX).unwrap(),
    )
}

type State = Arc<Mutex<StateImpl>>;

#[derive(serde::Serialize, serde::Deserialize)]
struct SimulationRun {
    /// The parameters used in this run
    parameters: IndexMap<String, f64>,
    /// The error score for this run
    #[serde(rename = "fitness")]
    error: f64,
    /// The time this run finished
    time: SystemTime,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StateImpl {
    /// The parameters in use. The `name` value in the Parameter struct corresponds with the
    /// key in a `SimulationRun`'s `parameters` map
    params: Vec<Parameter>,

    /// Finished runs
    results: Vec<SimulationRun>,
}

static RUNNING: AtomicBool = AtomicBool::new(true);
static PATH: OnceCell<String> = OnceCell::new();
static STATE: OnceCell<State> = OnceCell::new();

static BASE_ARGUMENTS: [&str; 5] = [
    "--duration=180",
    "--pNodes=8",
    "--packetInterval=0.3",
    "--calculateInterval=0.01",
    "--spawnRadius=8.5",
];
const TARGET_DISTANCE: f64 = 7.5;

const MAX_SIMULATIONS: usize = 1000;

static LOWEST_ERROR: atomic_float::AtomicF64 = atomic_float::AtomicF64::new(10000.0);

pub fn run(path: &str) {
    ctrlc::set_handler(|| {
        static FORCE_EXIT: AtomicUsize = AtomicUsize::new(0);
        let count = FORCE_EXIT.fetch_add(1, Ordering::Relaxed);
        if count == 5 {
            println!("Failed to stop after 5 tries, force quitting");
            std::process::exit(1);
        }
        RUNNING.store(false, Ordering::Relaxed);
        println!(" Shutting down runners");
    })
    .expect("failed to to set Control-C handler");

    let _ = STATE.set(Arc::new(Mutex::new(StateImpl {
        params: vec![
            Parameter {
                name: "a".to_owned(),
                optim: tpe::TpeOptimizer::new(
                    tpe::parzen_estimator(),
                    tpe::range(PARAM_MIN, PARAM_MAX).unwrap(),
                ),
            },
            Parameter {
                name: "r".to_owned(),
                optim: tpe::TpeOptimizer::new(
                    tpe::parzen_estimator(),
                    tpe::range(PARAM_MIN, PARAM_MAX).unwrap(),
                ),
            },
        ],
        results: Vec::new(),
    })));
    let default_error = LOWEST_ERROR.load(Ordering::Relaxed);
    for param in STATE.get().unwrap().lock().unwrap().params.iter_mut() {
        // Fill in default values so parameters start around 1 by default
        param.optim.tell(1.0, default_error).unwrap();
    }

    let mut threads = Vec::new();
    let _ = PATH.set(path.to_owned());
    for _ in 0..num_cpus::get() {
        //for _ in 0..1 {
        threads.push(std::thread::spawn(run_thread));
    }
    println!("Runners started");
    for thread in threads {
        let _ = thread.join();
    }

    println!("All runners stopped");
    let state = STATE.get().unwrap().lock().unwrap();
    println!("Exporting results from {} simulations", state.results.len());

    let json = serde_json::to_string(state.deref()).unwrap();
    let now = SystemTime::now();
    let delta = now
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    std::fs::write(format!("output-{}.json", delta.as_secs()), json)
        .expect("Failed to write stats to file");
    println!("Wrote data backup file");

    write_hot_cold(&state, "hot_cold.png").unwrap();
    write_error_time(&state, "error_time.png").unwrap();
}

pub fn re_export(json_path: impl AsRef<Path>, prefix: Option<&str>) -> Result<(), crate::Error> {
    let json = std::fs::read_to_string(json_path)?;
    let state: StateImpl = serde_json::from_str(&json)?;
    if state.results.len() < 1000 {
        println!(
            "WARN: only {} runs counted. Dataset might be too small",
            state.results.len()
        );
    }

    let hot_cold_path = format!("{}hot_cold.png", prefix.unwrap_or(""));
    write_hot_cold(&state, &hot_cold_path)?;

    let error_time_path = format!("{}error_time.png", prefix.unwrap_or(""));
    write_error_time(&state, &error_time_path)?;

    println!("Exported {} runs successfully", state.results.len());
    Ok(())
}

pub fn re_export_all(dir_path: impl AsRef<Path>) -> Result<(), crate::Error> {
    let path = dir_path.as_ref();
    println!("Checking {:?} for json files", path.to_str());
    for entry in walkdir::WalkDir::new(dir_path)
        .contents_first(true)
        .into_iter()
        .filter_entry(|e| {
            e.file_type().is_dir()
                || e.file_name()
                    .to_str()
                    .map(|s| s.ends_with(".json"))
                    .unwrap_or(false)
        })
        .flatten()
    {
        if entry.file_type().is_file() {
            let parent = entry.path().parent().expect("json file has no parent!");
            if let Err(err) = re_export(entry.path(), parent.to_str()) {
                println!(
                    "Failed to export {}: {:?}",
                    entry.path().to_str().unwrap(),
                    err
                );
            } else {
                println!("Exported {} successfully", entry.path().to_str().unwrap(),);
            }
        }
    }
    Ok(())
}

/// Returns the axis ranges for a set of points of which points within `range_include` standard
/// deviations of the mean are within the range
fn get_bounds_and_regression(
    points: &[(f64, f64, f64)], //(x, y, error)
    range_include: f64,
) -> (Range<f64>, Range<f64>, f64, f64) {
    //Clone points so we can work with a sorted version
    let mut points: Vec<_> = points.iter().collect();
    points.sort_by(|(_, _, error1), (_, _, error2)| error1.partial_cmp(error2).unwrap());
    //We need this to be sorted so we can do regression on only the best ones

    let x_coords: Vec<f64> = points.iter().map(|(x, _, _)| *x).collect();
    let y_coords: Vec<f64> = points.iter().map(|(_, y, _)| *y).collect();
    let weight: Vec<f64> = points
        .iter()
        // Weight is inversely positional to weight
        // low error -> high weight, high error -> low weight
        .map(|(_, _, error)| *error)
        .collect();

    // gsl requires this when we do `wlinear`
    assert!(x_coords.len() == weight.len());
    assert!(weight.len() == y_coords.len());

    let x_mean = rgsl::statistics::mean(&x_coords, 1, x_coords.len());
    let y_mean = rgsl::statistics::mean(&y_coords, 1, y_coords.len());
    let x_stddev = rgsl::statistics::sd(&x_coords, 1, x_coords.len());
    let y_stddev = rgsl::statistics::sd(&y_coords, 1, y_coords.len());

    // Only run regression on the 20% of the points with the lowest error
    // the lists are sorted so the best values are at the beginning
    const REGRESSION_INCLUDE_TOP_PERCENT: f64 = 0.1;
    let best_count = (x_coords.len() as f64 * REGRESSION_INCLUDE_TOP_PERCENT) as usize;
    for ((x, y), error) in x_coords
        .iter()
        .zip(y_coords.iter())
        .zip(weight.iter())
        .take(best_count)
    {
        println!("[{}, {}] = {}", x, y, error);
    }
    let (aa, b, m, bb, cc, dd, r_squared) =
        rgsl::fit::wlinear(&x_coords, 1, &weight, 1, &y_coords, 1, best_count);
    println!("Got y={}x + {}, r^2={}", m, b, r_squared);
    dbg!(aa, b, m, bb, cc, dd, r_squared);
    let x_min = (x_mean - x_stddev * range_include).max(PARAM_MIN);
    let x_max = (x_mean + x_stddev * range_include).min(PARAM_MAX);

    let y_min = (y_mean - y_stddev * range_include).max(PARAM_MIN);
    let y_max = (y_mean + y_stddev * range_include).min(PARAM_MAX);
    println!(
        "Bounds are x= {}..{}, y= {}..{}",
        x_min, x_max, y_min, y_max
    );

    (
        //clamp to the ranges that we simulated and apply `range_include`
        x_min..x_max,
        y_min..y_max,
        m,
        b,
    )
}

fn write_hot_cold(state: &StateImpl, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut error_scores: Vec<f64> = state
        .results
        .iter()
        .map(|r| r.error)
        .filter(|v| !v.is_nan())
        .collect();

    error_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

    //We want to color the points on the scatteragram based on the fitness for that point.
    //Finding the best fitness and assigning it green, and the worst fitness red, and then
    //interpolating the rest results is most of the points being green because the fitness scores
    //are not distributed evenly across the (best_fitness..worst_fitness) range.
    //To combat this we will make a frequency table so that fitness scores in the top 10-20% range
    //will be 80-90% green and the rest red. This will be repeated for all points with the step
    //size being 1/256 * 100 % to make it very smooth

    let step_size = 256;
    let smoother = crate::util::RangeSmoother::new(step_size, error_scores.as_slice());
    let smoothed_values: Vec<_> = smoother.ranges().collect();

    let mut params_to_draw: Vec<&String> = state.results[0].parameters.keys().take(2).collect();
    params_to_draw.sort_by(|a, b| b.cmp(a));
    let points: Vec<_> = state
        .results
        .iter()
        .map(|result| {
            let params_used = &result.parameters;
            let x = params_used[params_to_draw[0]];
            let y = params_used[params_to_draw[1]];
            (x, y, result.error)
        })
        .collect();

    let root = BitMapBackend::new(file_name, (1024, 768)).into_drawing_area();

    root.fill(&WHITE)?;
    let x_param = params_to_draw[0];
    let y_param = params_to_draw[1];

    // Disable title
    // root.titled(
    //     format!("{} vs. {}", x_param, y_param).as_str(),
    //     ("sans-serif", 40),
    // )?;
    //

    const INCLUDE_POINTS_STDDEVS: f64 = 1.0;
    let areas = root.split_by_breakpoints([944], [80]);
    let (x_bounds, y_bounds, linear_m, linear_b) =
        get_bounds_and_regression(&points, INCLUDE_POINTS_STDDEVS);

    let regression_func = |x: f64| -> f64 {
        let y = linear_m * x + linear_b;
        println!("f({}) = {}", x, y);
        y
    };

    let mut scatter_ctx = ChartBuilder::on(&areas[2])
        .x_label_area_size(60)
        .y_label_area_size(80)
        .build_cartesian_2d(x_bounds, y_bounds)?;

    scatter_ctx
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_desc(x_param)
        .y_desc(y_param)
        .label_style(("sans-serif", 30))
        .axis_desc_style(("sans-serif", 30))
        .draw()?;

    scatter_ctx
        .draw_series(LineSeries::new(
            (-50..=50).map(|x| x as f64).map(|x| (x, (x / 10.0).sqrt())),
            //vec![(0.0, regression_func(0.0)), (50.0, regression_func(50.0))].into_iter(),
            &BLUE,
        ))?
        .label("Average error");

    scatter_ctx.draw_series(points.iter().map(|(x, y, error)| {
        let mut i = 0;
        for limit in smoothed_values.iter() {
            i += 1;
            if error < limit {
                break;
            }
        }

        let color = plotters::style::RGBColor(i as u8, (256 - i) as u8, 50);
        Circle::new((*x, *y), 2, color.filled())
    }))?;

    root.present().expect("Unable to write image to file");

    Ok(())
}

fn write_error_time(state: &StateImpl, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(file_name, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;
    if state.results.is_empty() {
        println!("No data to graph");
        return Ok(());
    }
    let worst_error = state
        .results
        .iter()
        .map(|e| e.error)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap() as f32
        * 0.7; //Scale down to hide outliers

    let start = state.results.first().unwrap().time;
    let end = state.results.last().unwrap().time;

    let seconds_since_start = |time: &SystemTime| time.duration_since(start).unwrap().as_secs_f64();

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(65u32)
        .y_label_area_size(110u32)
        .build_cartesian_2d(0.0..(seconds_since_start(&end) as f32), 0f32..worst_error)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_desc("Time (s)")
        .y_desc("error")
        .label_style(("sans-serif", 25))
        .axis_desc_style(("sans-serif", 25))
        .light_line_style(&WHITE)
        .draw()?;

    chart.draw_series(state.results.iter().map(|r| {
        let a = (seconds_since_start(&r.time) as f32, r.error as f32);
        Circle::new(a, 2u32, &BLACK)
    }))?;

    chart
        .draw_series(LineSeries::new(
            state.results.chunks(num_cpus::get() * 3 / 2).map(|runs| {
                let x = runs
                    .iter()
                    .map(|r| seconds_since_start(&r.time) as f32)
                    .sum::<f32>()
                    / runs.len() as f32;
                let y = runs.iter().map(|r| r.error as f32).sum::<f32>() / runs.len() as f32;

                (x, y)
            }),
            &BLUE,
        ))?
        .label("Average error");

    Ok(())
}

fn run_binary(
    rel_working_dir: &str,
    rel_bin_path: &str,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut base = std::env::current_dir().unwrap();
    base.push(rel_working_dir);
    //We need the NS3 libs to be in LD_LIBRARY_PATH
    let lib_path = {
        let mut base = base.clone();
        base.push("build");
        base.push("lib");
        base
    };
    let current_dir = base.clone();
    base.push(rel_bin_path);
    let bin_path = base;

    if Command::new(bin_path)
        .current_dir(current_dir)
        .env("LD_LIBRARY_PATH", lib_path.to_str().unwrap())
        .args(args)
        .spawn()?
        .wait()?
        .success()
    {
        Ok(())
    } else {
        Err("Error running binary".into())
    }
}

fn run_thread() {
    let mut rng = rand::thread_rng();
    let mut param_map = IndexMap::new();
    let mut args: Vec<String> = Vec::new();
    for arg in BASE_ARGUMENTS.iter() {
        args.push((*arg).to_owned());
    }

    while RUNNING.load(Ordering::Relaxed) {
        let pos_file_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        //Keep base arguments
        args.resize(BASE_ARGUMENTS.len(), String::new());

        let ns3_path = PATH.get().unwrap();
        let mut buf = PathBuf::from(ns3_path);
        buf.push(pos_file_name);
        buf.set_extension("csv");
        let mut positions_file = std::env::current_dir().unwrap();
        positions_file.push(buf);
        args.push(format!(
            "--positionsFile={}",
            &positions_file.to_str().unwrap()
        ));

        let seed: usize = rng.gen();
        args.push(format!("--seed={}", seed));

        {
            let mut state = STATE.get().unwrap().lock().unwrap();
            param_map.clear();
            for param in state.params.iter_mut() {
                let value = param.optim.ask(&mut rng).unwrap();
                param_map.insert(param.name.clone(), value);
                args.push(format!("--{}={}", param.name, value));
            }
        };

        //Run simulation
        match run_binary(ns3_path, "build/scratch/non-ideal/non-ideal", &args) {
            Ok(_) => match run_analysis(&positions_file, &param_map, &positions_file) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error while doing analysis: {}", err);
                }
            },
            Err(err) => {
                println!("Error while running waf: {}", err);
                let _ = std::fs::remove_file(positions_file);
            }
        }
    }
    println!("Runner exiting cleanly");
}

fn get_error(data: &mut SimulationData) -> f64 {
    let time_step = 0.1;
    let mut time = 0.0;
    let mut last_poses = IndexMap::new();
    let uavs = data.uavs.clone();
    let central_node = uavs.iter().min().unwrap();

    let mut all_central_distances = Vec::new();
    let mut all_peripheral_distances = Vec::new();
    let mut all_velocities = Vec::new();
    while time <= data.simulation_length {
        let mut central_distances: Vec<f64> = Vec::new();
        let mut peripheral_distances: Vec<f64> = Vec::new();
        let mut velocities: Vec<f64> = Vec::new();

        let central_pos = data.pos_at_time(TimePoint(time), *central_node).unwrap();
        for uav in &uavs {
            if let Some(now_pos) = data.pos_at_time(TimePoint(time), *uav) {
                match last_poses.get(uav) {
                    None => {}
                    Some((last_pos, last_time)) => {
                        let pos_delta = now_pos - *last_pos;
                        let time_delta = time - last_time;
                        let velocity: Vec3A = pos_delta / time_delta;
                        velocities.push(velocity.length() as f64);
                    }
                }
                last_poses.insert(uav, (now_pos, time));
                if uav != central_node {
                    central_distances.push((now_pos - central_pos).length() as f64);
                    for uav_2 in &uavs {
                        if uav != uav_2 && uav_2 != central_node {
                            //Calculate the distance between this node and every other peripheral node
                            if let Some(now_2_pos) = data.pos_at_time(TimePoint(time), *uav_2) {
                                peripheral_distances.push((now_2_pos - now_pos).length() as f64);
                            }
                        }
                    }
                }
            }
        }

        let central_distances_mean =
            rgsl::statistics::mean(&central_distances, 1, central_distances.len());
        let peripheral_distances_mean =
            rgsl::statistics::mean(&peripheral_distances, 1, peripheral_distances.len());

        let mean_velocity = rgsl::statistics::mean(&velocities, 1, velocities.len());

        all_central_distances.push(central_distances_mean);
        all_velocities.push(mean_velocity);
        all_peripheral_distances.push(peripheral_distances_mean);
        //println!("T: {}, V: {}, D: {}", time, mean_velocity, mad_of_distance);

        time += time_step;
    }
    let mean_velocity: f64 = all_velocities.iter().sum::<f64>() / all_velocities.len() as f64;

    let mean_central_distance: f64 =
        rgsl::statistics::mean(&all_central_distances, 1, all_central_distances.len());

    let mad_of_peripheral_distance: f64 =
        rgsl::statistics::absdev(&all_peripheral_distances, 1, all_peripheral_distances.len());

    println!("mean central: {mean_central_distance}, c mad: {mad_of_peripheral_distance}");

    let p_mad_cost = 400.0 * mad_of_peripheral_distance;
    let central_distance_cost = 400.0 * (TARGET_DISTANCE - mean_central_distance).abs();
    let velocity_cost = 250.0 * mean_velocity;

    p_mad_cost + central_distance_cost + velocity_cost
}

fn run_analysis(
    pos_path: &std::path::Path,
    param_map: &IndexMap<String, f64>,
    positions_file: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    //let start = Instant::now();
    let positions = String::from_utf8(std::fs::read(&pos_path)?)?;
    let mut data = SimulationData::parse(&positions)?;
    let error = get_error(&mut data);
    {
        let mut state = STATE.get().unwrap().lock().unwrap();
        for param in state.params.iter_mut() {
            let value = param_map.get(&param.name).unwrap();
            param.optim.tell(*value, error).unwrap();
        }
        state.results.push(SimulationRun {
            parameters: param_map.clone(),
            time: SystemTime::now(),
            error,
        });
        let simulations = state.results.len();
        if simulations == MAX_SIMULATIONS {
            println!("Exiting after {}", MAX_SIMULATIONS);
            RUNNING.store(false, Ordering::Relaxed);
        } else {
            println!("  {}", simulations);
        }
    }
    let old_error = LOWEST_ERROR.load(Ordering::Relaxed);
    if error < old_error {
        //If multiple threads get in here we don't really care...
        LOWEST_ERROR.store(error, Ordering::Relaxed);
        let src = positions_file;
        let mut dest = PathBuf::from(positions_file);
        dest.pop(); //Pop positions csv file name
        dest.push("out");
        let _ = std::fs::create_dir_all(&dest);
        dest.push(format!("{}.csv", error));
        std::fs::copy(src, dest).unwrap();
        println!("  got best error: {} for params: {:?}", error, param_map);
    }

    if let Some(err) = std::fs::remove_file(pos_path).err() {
        println!(
            "failed to delete temp positions file: {} - {}",
            pos_path.to_str().unwrap(),
            err
        );
    }
    Ok(())
}
