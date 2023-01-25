use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*, BufReader, stdin, Write, BufWriter};
use std::time::Instant;

use itertools::Itertools;
use linya::{Bar, Progress};
use chrono;
use rustdate::update::UpdateBuilder;
use rustdate::utility::OsType;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUFFER: i32 = i32::pow(2, 14);
const LIMIT: usize = 20000000;

const PRESSURE_THRESHOLD: f32 = 101.;
const PRESSURE_FIELD: &'static str = "F_pri_pressure_bar";
const DISPLACEMENT_FIELD: &'static str = "Displacement_A_mm";

type Dataset = HashMap<usize, ([f32; 2], usize)>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    UpdateBuilder::new()
        .set_verbose(true)
        .set_github_user("TuuKeZu")
        .set_github_repo("github-actions")
        .set_binary_path(OsType::Windows, "x86_64-pc-windows-gnu.zip")
        .set_binary_path(OsType::Linux, "x86_64-unknown-linux-musl.zip")
        .check_for_updates()
        .await?;

    println!("tAnalyzer v{}", VERSION);
    println!("<github>");
    println!("---------------");
    println!("Please input the name of the input file [.trd]:");

    let mut file_name = String::new();
    stdin().read_line(&mut file_name)?;

    let file = File::open(format!("input/{}", file_name.trim().clone()))?;
    let metadata = file.metadata().unwrap();
    let size = metadata.len();
    println!("---------------");
    println!("File:");
    println!("> name: {}", file_name.trim());
    println!("> size: {} bytes", size);
    println!("> type: {:?}", metadata.file_type());
    println!("Constants:");
    println!("> buffer size: {:#?} bytes", BUFFER);
    println!("> pressure threshold: {} bar", PRESSURE_THRESHOLD);
    println!("> pressure field: {:?}", PRESSURE_FIELD);
    println!("> displacement field: {:?}", DISPLACEMENT_FIELD);
    println!("---------------");
    println!("Proceed? [enter / ctrl + c]:");
    stdin().read_line(&mut String::new())?;
    println!("> Ok");

    let dataset = map_data(file)?;
    
    write_file(dataset)?;

    println!("---------------");
    println!("Saved output.trd");
    println!("Done!");

    Ok(())
}

fn map_data(file: File) -> Result<Dataset, io::Error> {
    let metadata = file.metadata().unwrap();
    let size = metadata.len();

    let t_start = Instant::now();
    // Time the execution time
    let mut last : Option<f32> = None;
    let mut relative_highest: [f32; 2] = [0., 0.];
    let mut j: usize = 1;
    
    let mut dataset: Dataset = HashMap::new();
    
    let reader = BufReader::with_capacity(BUFFER as usize, file);
    println!("> Initilized BufReader with {} bytes of memory", BUFFER);

    let mut progress = Progress::new();
    let bar: Bar = progress.bar(size as usize / 260, "> Analyzing");


    for (i, line) in reader.lines().enumerate() {
        let data = line?;
        let set = data.split(";").collect::<Vec<&str>>();
        let mut chunks = set.chunks(2);

        if last.is_none() {
            let pr = chunks.find(|pair| pair.get(0).unwrap() == &PRESSURE_FIELD).unwrap().get(1).unwrap().parse::<f32>().unwrap();
            if pr > PRESSURE_THRESHOLD {
                let v = chunks.find(|pair| pair.get(0).unwrap() == &DISPLACEMENT_FIELD).unwrap().get(1).unwrap().parse::<f32>().unwrap();
                last = Some(v);
            }

        } else {
            let value = chunks.find(|pair| pair.get(0).unwrap() == &DISPLACEMENT_FIELD).unwrap().get(1).unwrap().parse::<f32>().unwrap();
            let a = if value.is_sign_positive() {relative_highest.get_mut(0).unwrap()} else {relative_highest.get_mut(1).unwrap()};

            if f32::abs(value) > f32::abs(*a) {
                *a = value;
            }
            
            //let _cycle = (j as f32) * 0.2 / 33.33 % 1.;

            if last.unwrap().is_sign_positive() && value.is_sign_negative() {
                let count = dataset.keys().len() + 1;
                if j > 150 {

                    if relative_highest.iter().all(|x| x != &0.) {
                        dataset.insert(count, (relative_highest, j));
                        relative_highest = [0., 0.];
                    }
                }

                j = 3;

            }
            
            last = Some(value);
            j += 1;

            if i % (LIMIT / 20) == 0 {
                progress.set_and_draw(&bar, i);
            }
            
            if i > LIMIT {
                break;
            }
            
        }
    }
    println!("---------------");
    println!("> Analyzed the data in {}ms", t_start.elapsed().as_millis());

    Ok(dataset)
}

fn write_file(dataset: Dataset) -> Result<(), io::Error> {
    let size = dataset.len();
    let d = if size < 20 {size} else {20};

    let file = File::create(format!("output/output-{}.trd", chrono::offset::Local::now().format("%Y-%m-%d-%H-%M-%S")))?;
    let mut writer = BufWriter::with_capacity(BUFFER as usize, file);

    println!("> Initilized BufWriter with {} bytes of memory", BUFFER);

    let mut progress = Progress::new();
    let bar: Bar = progress.bar(size, "> Writing output.trd");
    
    for (i, k) in dataset.keys().sorted().enumerate() {
        let val = dataset.get(k).unwrap();
        writer.write(format!("{};min;{};max;{};cycles;{};\n", i + 1, val.0.get(0).unwrap(), val.0.get(1).unwrap(), val.1).as_bytes())?;

        if i % (size / d) == 0 {
            progress.set_and_draw(&bar, i + 1);
        }
    };
    Ok(())
}
