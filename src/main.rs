pub mod utility;
pub mod analyzing;


use core::panic;
use std::any::{self, Any};
use std::fs::{File, self};
use std::io::{self, stdin, Write, BufWriter};
use std::time::Duration;
use itertools::Itertools;
use linya::{Bar, Progress};
use chrono;
use rustdate::update::UpdateBuilder;
use rustdate::utility::{OsType, Data};
use tokio::sync::RwLock;

use crate::analyzing::{Dataset, BUFFER};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BACK_KEY: &str = "b";

type State = StateChange<dyn Any>;

#[derive(Debug)]
pub struct StateChange<T : ?Sized> {
    pub change: Option<ApplicationState>,
    pub value: Box<T>
}

impl StateChange<dyn Any> {
    pub fn new<T : 'static>(value: T) -> Self {
        Self {
            change: None,
            value: Box::new(value)
        }
    }

    pub fn switch<T : 'static>(value: T, state: ApplicationState) -> Self {
        Self {
            change: Some(state),
            value: Box::new(value)
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApplicationState {
    Menu,
    Selecting,
    MinMax,
    Close,
}

impl ApplicationState {
    pub fn menu_options() -> Vec<ApplicationState> {
        vec![
            Self::MinMax,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ApplicationState::Menu => "Back to Menu",
            ApplicationState::Selecting => "Open a different File",
            ApplicationState::Close => "Exit the analyzer",
            ApplicationState::MinMax => "Analyze minimium and maximium values",
            _ => ""
        }
    }
}

pub struct Application {
    pub state: RwLock<ApplicationState>,
    pub open_file: Option<String>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ApplicationState::Selecting),
            open_file: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        clear();
        self.handle_update().await?;
        std::thread::sleep(Duration::from_secs(1));

        loop {
            let mut state_lock = self.state.write().await;
            
            match *state_lock {
                ApplicationState::Menu => {
                    let a = self.menu().await?;

                    if let Some(target) = a.change {
                        *state_lock = target;
                    }
                },
                ApplicationState::Selecting => {
                    let a = self.select_file().await?;

                    let path = a.value.downcast_ref::<String>().unwrap();
                    if !path.is_empty() { self.open_file = Some(path.clone()); }

                    if let Some(target) = a.change {
                        *state_lock = target;
                    }
                },
                ApplicationState::MinMax => {
                    if let Some(path) = self.open_file.clone() {
                        let a = self.min_max_data(path).await?;

                        if let Some(target) = a.change {
                            *state_lock = target;
                        }
                    }
                },
                ApplicationState::Close => { break; },
            }

            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    pub async fn handle_update(&self) -> anyhow::Result<()> {
        UpdateBuilder::new()
            .set_verbose(true)
            .set_github_user("TuuKeZu")
            .set_github_repo("data-sampler")
            .set_binary_path(OsType::Windows, "x86_64-pc-windows-gnu.zip")
            .set_binary_path(OsType::Linux, "x86_64-unknown-linux-musl.zip")
            .check_for_updates()
            .await?;

        Ok(())
    }

    pub async fn menu(&self) -> anyhow::Result<State> {
        clear();
        
        println!("Current file: {}", self.open_file.as_ref().unwrap());
        let count = ApplicationState::menu_options().len();
        
        for(i, option) in ApplicationState::menu_options().iter().map(|o| o.label()).enumerate() {
            println!("[{}]: {}", (i + 1), option);
        }
        
        back();
        println!("---------------");
    
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        if input.trim() == BACK_KEY { return Ok(StateChange::switch((), ApplicationState::Selecting)); }
        
        let mut idx = input.trim().parse::<usize>();
    
        while !input.is_empty() && (idx.is_err() || !(1..count + 1).contains(&idx.clone().unwrap())) {
            println!("Please enter a valid key");
    
            input = String::new();
            stdin().read_line(&mut input)?;

            if input.trim() == BACK_KEY { return Ok(StateChange::switch((), ApplicationState::Selecting)); }

            idx = input.trim().parse::<usize>();
        }

        Ok(StateChange::switch((), ApplicationState::MinMax))
    }

    async fn select_file(&self) -> anyhow::Result<State> {
        clear();

        let files = fs::read_dir("input/")?;
        let mut count = 0;
        let mut paths: Vec<String> = Vec::new();
    
        println!("Please select the file you want to analyze from '/input' folder");
        println!("---------------");
        
        for (i, file) in files.enumerate() {
            let data = file?.path();
            let path = data.to_str().unwrap().to_string();
            println!("> [{i}]: {}", &path);
            paths.push(path);
            
            count += 1;
        }
        
        if count == 0 {
            println!("No Files found");
            return Ok(StateChange::switch(String::new(), ApplicationState::Menu));
        }
    
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        
        let idx = input.trim().parse::<usize>();
        while idx.clone().is_err() || idx.clone().unwrap() > count - 1 {
            println!("> The input must be a valid integer");
            let mut input = String::new();
            stdin().read_line(&mut input)?;
        }
    
        Ok(StateChange::switch(paths[idx.unwrap()].clone(), ApplicationState::Menu))
    }

    pub async fn min_max_data(&self, path: String)-> anyhow::Result<State> {
        clear();

        let data: Dataset = analyzing::map_data(path)?;
        analyzing::write_file(data)?;

        println!("> Successfully analyzed file.");
        println!("[Press any key to continue]");

        let mut input = String::new();
        stdin().read_line(&mut input)?;


        Ok(StateChange::switch((), ApplicationState::Menu))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Application::new();
    app.run().await?;

    Ok(())
}

fn back() {
    println!("[{}] {}", BACK_KEY, ApplicationState::Selecting.label());
}

fn clear() {
    print!("{}[2J", 27 as char);
    println!("tAnalyzer v{}", VERSION);
    println!("---------------");
}
