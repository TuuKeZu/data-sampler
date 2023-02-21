pub mod utility;

use std::collections::HashMap;
use std::fs::{File, self};
use std::io::{self, prelude::*, BufReader, stdin, Write, BufWriter};
use std::time::Instant;

use itertools::Itertools;
use linya::{Bar, Progress};
use chrono;
use rustdate::update::UpdateBuilder;
use rustdate::utility::OsType;

use crate::utility::get_config;


const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUFFER: i32 = i32::pow(2, 14);

const MAX: usize = 2000;

type Dataset = HashMap<usize, (MinMaxValue, usize)>;


struct MinMaxValue {
    min: f32,
    max: f32
}

impl Default for MinMaxValue {
    fn default() -> Self {
        Self {
            min: f32::INFINITY,
            max: -f32::INFINITY
        }
    }
}

impl MinMaxValue {
    /// Insert value if it's greater than max or lesser than min
    fn insert(&mut self, value: f32) { 
        if value > self.max {
            self.max = value;
        }

        if value < self.min {
            self.min = value;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    UpdateBuilder::new()
        .set_verbose(true)
        .set_github_user("TuuKeZu")
        .set_github_repo("data-sampler")
        .set_binary_path(OsType::Windows, "x86_64-pc-windows-gnu.zip")
        .set_binary_path(OsType::Linux, "x86_64-unknown-linux-musl.zip")
        .check_for_updates()
        .await?;

    println!("tAnalyzer v{}", VERSION);
    println!("<https://github.com/TuuKeZu/data-sampler.git>");
    println!("---------------");
    let path = select_file()?;

    if path.is_none() {
        println!("> No files found in '/input' folder");
        return Ok(());
    } 

    let dataset = map_data(path.unwrap())?;
    write_file(dataset)?;

    println!("---------------");
    println!("Saved output.trd");
    println!("Done!");
    
    Ok(())
}

fn select_file() -> Result<Option<String>, io::Error> {
    let files = fs::read_dir("input/")?;
    let mut count = 0;
    let mut paths: Vec<String> = Vec::new();

    println!("Please select the file you want to analyze from '/input' folder");
    println!("---------------");

    for (i, file) in files.enumerate() {
        let data = file?.path();
        let path = data.to_str().unwrap().to_string();
        println!("> {i}: {}", &path);
        paths.push(path);

        count += 1;
    }

    if count == 0 {
        return Ok(None);
    }

    if count == 1 {
        return Ok(Some(paths[0].clone()));
    }

    println!("---------------");
    println!("Enter an integer between [0..{}]", count - 1);

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    let idx = input.trim().parse::<usize>();

    if idx.is_err() {
        println!("> The input must be a valid integer");
        return Ok(None);
    }
    let idx = idx.unwrap();

    if idx > count - 1 {
        println!("> The input must be in range [0..{}]", count - 1);
        return Ok(None);
    }

    Ok(Some(paths[idx].clone()))
}

fn map_data(path: String) -> Result<Dataset, io::Error> {
    let config = get_config()?;
    println!("---------------");
    println!("> Displacement field: {:#?}", config.displacement_field);
    println!("> Pressure field: {:#?}", config.pressure_field);
    println!("> [min-max] will be calculated for: {:#?}", config.min_max_field);
    println!("> Pressure threshold: {:#?}", config.pressure_threshold);
    println!("---------------");

    println!("> Opening {}", &path);
    let file = File::open(path.clone())?;

    let lines = linecount::count_lines(file)?;
    println!("> Found {} datapoints", lines);

    let file = File::open(path)?;

    let t_start = Instant::now();
    // Time the execution time

    let mut last : Option<f32> = None;
    let mut relative_highest = MinMaxValue::default();
    let mut j: usize = 1;
    
    let mut dataset: Dataset = HashMap::new();
    
    let reader = BufReader::with_capacity(BUFFER as usize, file);

    let mut progress = Progress::new();
    let bar: Bar = progress.bar(lines, "> Analyzing");


    for (i, line) in reader.lines().enumerate() {
        let data = line?;
        let set = data.split(";").collect::<Vec<&str>>();
        let mut chunks = set.chunks(2);

        if last.is_none() {
            let pr = chunks.find(|pair| pair.get(0).unwrap() == &config.pressure_field).unwrap().get(1).unwrap().parse::<f32>().unwrap();
            if pr > config.pressure_threshold {
                let displacement = chunks.find(|pair| pair.get(0).unwrap() == &config.displacement_field).unwrap().get(1).unwrap().parse::<f32>().unwrap();
                last = Some(displacement);
            }

        } else {
            let displacement = chunks.clone().find(|pair| pair.get(0).unwrap() == &config.displacement_field).unwrap().get(1).unwrap().parse::<f32>().unwrap();
            let value = chunks.clone().find(|pair| pair.get(0).unwrap() == &config.min_max_field).unwrap().get(1).unwrap().parse::<f32>().unwrap();

            
            relative_highest.insert(value);
            
            //let _cycle = (j as f32) * 0.2 / 33.33 % 1.;

            if last.unwrap().is_sign_positive() && displacement.is_sign_negative() {
                let count = dataset.keys().len() + 1;
                if j > 150 {
                    dataset.insert(count, (relative_highest, j));
                    relative_highest = MinMaxValue::default();
                }

                j = 3;

            }
            
            last = Some(displacement);
            j += 1;

            if i % (lines / 20) == 0 {
                progress.set_and_draw(&bar, i);
            }
            
        }
    }
    progress.set_and_draw(&bar, lines);

    println!("---------------");
    println!("> Analyzed the data in {}ms", t_start.elapsed().as_millis());

    Ok(dataset)
}

fn write_file(dataset: Dataset) -> Result<(), io::Error> {
    let size = dataset.len();
    let d = if size < 20 {size} else {20};

    let file = File::create(format!("output/output-{}.trd", chrono::offset::Local::now().format("%Y-%m-%d-%H-%M-%S")))?;
    let mut writer = BufWriter::with_capacity(BUFFER as usize, file);

    let mut progress = Progress::new();
    let bar: Bar = progress.bar(size, "> Writing output.trd");
    
    for (i, k) in dataset.keys().sorted().enumerate() {
        let val = dataset.get(k).unwrap();
        writer.write(format!("{};min;{};max;{};cycles;{};\n", i + 1, val.0.min, val.0.max, val.1).as_bytes())?;

        if i % (size / d) == 0 {
            progress.set_and_draw(&bar, i + 1);
        }
    };

    progress.set_and_draw(&bar, size);
    Ok(())
}
