use std::env::args;
use chrono::prelude::{DateTime, Local};
use utils::file_io::SafeFileEdit;

mod units;
use crate::units::day::{
    Day,
    create_daily_dir_if_not_exists, 
    get_current_day,
    read_day,
    write_day};

mod utils;
use crate::utils::file_io::{create_base_dir_if_not_exists};
use crate::utils::config::{Config, create_default_config_if_not_exists, get_config, update_config};


#[derive(PartialEq)]
enum SubCommand {
    In(Vec<String>),
    Out(Vec<String>),
    Pause(Vec<String>),
    Resume(Vec<String>),
    Summary(Vec<String>),
    View(Vec<String>),
    Edit(Vec<String>),
    Note(Vec<String>),
    EditConfig(Vec<String>),
    ViewConfig(Vec<String>),
    AddSummary(Vec<String>),
    Invalid(String),
}

impl SubCommand {
    fn from_string(name: &String, other_args: Vec<String>) -> Self {
        return match name.to_owned().trim() {
            "in" => Self::In(other_args),
            "out" => Self::Out(other_args),
            "pause" => Self::Pause(other_args),
            "resume" => Self::Resume(other_args),
            "summary" => Self::Summary(other_args),
            "view" => Self::View(other_args),
            "edit" => Self::Edit(other_args),
            "note" => Self::Note(other_args),
            "edit-config" => Self::EditConfig(other_args),
            "view-config" => Self::ViewConfig(other_args),
            "add-summary" => Self::AddSummary(other_args),
            other => Self::Invalid(other.to_string()),
        }
    }

    fn get_allowed_strings() -> Vec<String> {
        return Vec::from(
            [
                "in", "out", "pause", "resume", "summary", "view", "edit", "note", "edit-config", "add-summary"
            ].map(|x: &str| x.to_string())
        );
    }
}

fn main() {
    let env_args: Vec<String> = args().collect();
    let command_name: &String = &env_args[1];
    let other_args: Vec<String> = env_args[2..].to_vec();
    let command: SubCommand = SubCommand::from_string(command_name, other_args);

    setup();

    let now: DateTime<Local> = Local::now();
    run_command(command, now);
}

fn setup() {
    create_base_dir_if_not_exists();
    create_daily_dir_if_not_exists();
    create_default_config_if_not_exists();
}

fn run_command(command: SubCommand, now: DateTime<Local>) {
    if let SubCommand::In(other_args) = command {
        punch_in(&now, other_args);
    }
    else if let SubCommand::Invalid(original) = command {
        handle_invalid_cmd(&original);
    }
    else {
        let possible_day: Result<Day, String> = get_current_day(&now);
        if let Err(msg) = possible_day {
            println!("{}", msg);
            return
        }
        let day: Day = possible_day.unwrap();

        match command {
            SubCommand::Out(_) => punch_out(&now, day),
            SubCommand::Pause(_) => take_break(&now, day),
            SubCommand::Resume(_) => resume(&now, day),
            SubCommand::Summary(_) => summary(&now, day),
            SubCommand::View(_) => view_day(day),
            SubCommand::Edit(_) => edit_day(day),
            SubCommand::EditConfig(_) => edit_config(),
            SubCommand::ViewConfig(_) => view_config(),
            SubCommand::Note(other_args) => add_note_to_today(&now, day, other_args),
            SubCommand::AddSummary(other_args) => add_summary_to_today(day, other_args),
            SubCommand::In(_) => unreachable!("'punch in' commands shouldn't be being processed"),
            SubCommand::Invalid(_) => unreachable!("Invalid commands shouldn't be being processed here"),
        }
    }
}

fn punch_in(now: &DateTime<Local>, other_args: Vec<String>) {
    if let Ok(_) = read_day(now) {
        println!("You've already clocked in for the day!");
    }
    else{
        let parsed_args: (String, u64) = get_other_args_for_punch_in(other_args);
        let new_day: Day = Day::new(&now, parsed_args.0, parsed_args.1);
        println!("Clocking in for the day at '{}'", &new_day.get_day_start_as_str());
        write_day(&new_day);
    }
}

fn get_other_args_for_punch_in(other_args: Vec<String>) -> (String, u64) {
    let default_time_to_do: u64 = get_default_day_in_minutes();
    println!("Using the default time to do for the day: {}", default_time_to_do);
    let punch_in_task: String; 
    if other_args.len() == 0 {
        punch_in_task = get_default_punch_in_task();
        println!("No start task for the day provided. Using the default value.");
    }
    else {
        punch_in_task = other_args[0];
    }
    println!("Remember: You can use `punch edit` to change anything about the day.");
    return (punch_in_task, default_time_to_do)

}

fn get_default_day_in_minutes() -> u64 {
    return get_config().day_in_minutes() as u64;
}

fn get_default_punch_in_task() -> String {
    return get_config().default_punch_in_task.to_owned();
}

fn handle_invalid_cmd(command: &String) {
    println!("'{}' is not a valid subcommand for punch. Try one of the following:", command);
    for str_subcommand in SubCommand::get_allowed_strings() {
        println!("\t{}", str_subcommand);
    }
}

fn punch_out(now: &DateTime<Local>, mut day: Day) {
    if let Ok(_) = day.end_day_at(&now) {
        println!("Punching out for the day at '{}'", &day.get_day_end_as_str().unwrap().trim());
        write_day(&day);
        update_time_behind(day)
    }
    else {
        println!("Can't punch out: Already punched out for the day!")
    }
}

fn take_break(now: &DateTime<Local>, mut day: Day) {
    let break_result: Result<(), &str> = day.start_break_at(&now);
    if let Ok(_) = break_result {
        println!("Taking a break at '{}'", &now);
        write_day(&day);

        if !day.has_ended() {day.end_day_at(&now).expect("We should be able to end the day");}
        let mut config: Config = get_config();
        summarise_time(&day, &mut config);
    }
    else {
        let msg = break_result.unwrap_err();
        println!("{}", msg);
    }
}

fn resume(now: &DateTime<Local>, mut day: Day) {
    let resume_result: Result<(), &str> = day.end_current_block_at(&now);
    if let Ok(_) = resume_result {
        println!("Back to work at '{}'", &now);
        write_day(&day);
        if !day.has_ended() {day.end_day_at(&now).expect("We should be able to end the day");}
        let mut config: Config = get_config();
        summarise_time(&day, &mut config);
    }
    else {
        let msg = resume_result.unwrap_err();
        println!("{}", msg);
    }
}

fn view_day(day: Day) {
    println!("Here's the day so far: \n");
    println!("{}", day.as_string());
}

fn view_config() {
    println!("Here's the current config: \n");
    let config: Config = get_config();
    println!("{}", config.as_string());
}

fn edit_day(day: Day) {
    day.safe_edit_from_file();
}

fn edit_config() {
    let config = get_config();
    config.safe_edit_from_file();
}


fn summary(now: &DateTime<Local>, mut day: Day) {
    let end_result: Result<(), &str> = day.end_day_at(&now);
    match end_result {
        Ok(_) => (),
        _ => (),
    }
    let mut config: Config = get_config();
    summarise_time(&day, &mut config);
}


fn add_note_to_today(now: &DateTime<Local>, mut day: Day, other_args: Vec<String>) {
    if other_args.len() == 0 {
        println!("'punch note' requires a msg argument!")
    }
    else if other_args.len() > 1 {
        println!("'punch note' takes a single argument. Consider wrapping your message in quotes.")
    }
    else {
        let msg: String = (&other_args[0]).to_string();
        day.add_note(now, &msg);
        write_day(&day);
        println!("New note '{}' added to today at '{}'.", msg, now);
    }
}

fn add_summary_to_today(mut day: Day, other_args: Vec<String>) {
    if other_args.len() != 4 {
        println!("'punch add-summary' takes exactly 4 arguments: category, project, task and summary.")
    }
    else {
        let (category, project, task, summary) = (
            other_args[0].to_string(), other_args[1].to_string(), other_args[2].to_string(), other_args[3].to_string()
        );
        day.add_summary(category, project, task, summary);
        write_day(&day);
    }
}


fn summarise_time(day: &Day, config: &mut Config) {
    let time_left: i64 = day.get_time_left().expect("Day is over so we should be able to calculate time left!");
    let break_time: i64 = day.get_total_break_time().expect("Day is over so we should be able to calculate total break time!");
    config.update_minutes_behind(time_left);

    println!("Time done today: {}", day.get_time_done().unwrap());
    println!("Total time spent on break: {}", break_time);
    println!("Time left today: {}", time_left);
    println!("Minutes behind overall: {}", config.minutes_behind());
    println!("Minutes behind since last fall behind: {}", config.minutes_behind_non_neg());
}


fn update_time_behind(day: Day) {
    if day.has_ended() {
        let mut config: Config = get_config();
        summarise_time(&day, &mut config);
        update_config(config);
    }
    else {
        panic!("Can't update time behind: The day isn't over yet")
    }
}
