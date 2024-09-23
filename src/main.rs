use std::fs::File;
use std::vec::Vec;
use std::option::Option;
use std::env;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::clone::Clone;
use std::cmp::{PartialEq,Eq};
use inotify::{Inotify,WatchMask};
use chrono::{prelude::*, DateTime, NaiveDateTime, NaiveDate, NaiveTime};
use eframe::egui;
use eframe::egui::{Style, Visuals};

struct Assignment {
	due_date: DateTime<Local>,
	classname: String,
	name: String,
}

impl Assignment {
	fn load_from_file<P>(path: P) -> Vec<Assignment>
		where P: AsRef<Path>
	{
		// we load the file before waiting on a modification
		let mut assignments = Vec::new();

		// reload all assignments from file
		let list_file = File::open(path).unwrap();
		let list_reader = BufReader::new(list_file);
		for line in list_reader.lines() {
			let line = line.unwrap();

			let values = line.split(",").collect::<Vec<_>>();
			if values.len() >= 4 {
				let date = NaiveDate::parse_from_str(values[0], "%Y-%m-%d").unwrap();
				let time = NaiveTime::parse_from_str(values[1], "%H:%M").unwrap();
				assignments.push(Assignment {
					due_date: NaiveDateTime::new(date, time)
						.and_local_timezone(Local)
						.unwrap(),
					classname: String::from(values[2]),
					name: String::from(values[3]),
				});
			}
		}

		// sort by classname & due date
		assignments.sort_by(|a,b| {
			let a_due = a.due_date.timestamp();
			let b_due = b.due_date.timestamp();
			if a.classname == b.classname {
				return a_due.cmp(&b_due);
			}

			return a.classname.cmp(&b.classname);
		});

		return assignments;
	}
}

impl Clone for Assignment {
	fn clone(&self) -> Self {
		return Self {
			due_date: self.due_date.clone(),
			classname: self.classname.clone(),
			name: self.name.clone(),
		};
	}
}

impl PartialEq for Assignment {
	fn eq(&self, other: &Self) -> bool {
		return self.due_date == other.due_date &&
			self.classname == other.classname &&
			self.name == other.name;
	}
}

impl Eq for Assignment {
}

fn main() {
	let listpath = env::var("HOME").unwrap() + "/.todolist";
	let mut assignments = Assignment::load_from_file(&listpath);
	let eframe_options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1024.0, 724.0]),
		..Default::default()
	};
	
	let mut inotify = Inotify::init()
		.expect("Failed to initialize inotify");

	inotify
		.watches()
		.add(
			&listpath,
			WatchMask::MODIFY
		)
		.expect("Failed to add file watch to todolist");

	let mut buffer = [0; 64];

	eframe::run_simple_native("TODO List", eframe_options, move |ctx, _frame| {
		let style = Style {
			visuals: Visuals::light(),
			..Style::default()
		};
		ctx.set_style(style);

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("TODO List");
			ui.vertical(|ui| {
				if let Ok(events) =  inotify.read_events(&mut buffer) {
					if events.peekable().peek().is_some() {
						println!("Loading ~/.todolist");
						assignments = Assignment::load_from_file(&listpath);
					}
				}

				if assignments.is_empty() {
					ui.heading("Nothing due!");
				}
				else {
					let now = Local::now();
					let mut prev_class = String::from("");
					for assign in &assignments {
						let offset = (assign.due_date - now).num_seconds();

						// only write if the deadline hasn't passed, or if it's not more than a week away from today
						if offset >= 0 && offset < 60 * 60 * 24 * 7 {
							//println!("{} {} {}", assign.due_date.format("Due %B %e, %l:%M %p"), assign.name, assign.classname);
							if prev_class != assign.classname {
								ui.heading(&assign.classname);

								prev_class = assign.classname.clone();
							}

							ui.label(format!("{:<20} {}\n", assign.name, assign.due_date.format("due %b %e, %l:%M %p")));
						}
					}
				}
			});
		});
	}).unwrap();
}
