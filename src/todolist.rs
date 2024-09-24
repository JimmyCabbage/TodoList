use std::vec::Vec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, BufWriter};
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime};

use crate::assignment::Assignment;

pub struct TodoList {
	// class name to assignments
	pub assignments_by_class: HashMap<String, Vec<Assignment>>,
	assign_dir: PathBuf,
}

impl TodoList {
	pub fn new<P>(load_dir: P) -> Result<Self,&'static str>
		where P: AsRef<Path>
	{
		if !load_dir.as_ref().is_dir() {
			return Err("load_dir is not a directory");
		}

		let mut assignments_by_class = HashMap::new();

		// each subdir within the main dir represents a subject (name)
		// each sub-subdir represents an todo-item (name is assignment name)
		// within sub-subdir, date, time, and completed (optional)
		fs::read_dir(load_dir.as_ref())
			.unwrap()
			.filter_map(|e| e.ok())
			.filter(|e| e.file_type().unwrap().is_dir())
			.for_each(|e| {
				let classname = e.file_name()
					.to_str()
					.unwrap()
					.to_string();
				assignments_by_class.insert(classname.clone(), vec![]);
				let assignments = assignments_by_class.get_mut(&classname).unwrap();

				fs::read_dir(e.path())
					.unwrap()
					.filter_map(|e| e.ok())
					.filter(|e| e.file_type().unwrap().is_dir())
					.for_each(|e| {
						let name = e.file_name().as_os_str().to_str().unwrap().to_string();
						let mut date = None;
						let mut time = None;
						//let mut complete = None;

						fs::read_dir(e.path())
							.unwrap()
							.filter_map(|e| e.ok())
							.filter(|e| e.file_type().unwrap().is_file())
							.for_each(|e| {
								let path = e.path();
								match e.file_name().to_str().unwrap() {
									"date" => {
										let file_data = TodoList::read_file_sans_newline(path);
										date = NaiveDate::parse_from_str(&file_data, "%Y-%m-%d").ok();
									}
									"time" => {
										let file_data = TodoList::read_file_sans_newline(path);
										time = NaiveTime::parse_from_str(&file_data, "%H:%M").ok();
									}
									_ => (),
								};
							});

						if let (Some(good_date), Some(good_time)) = (date, time) {
							let due_date = NaiveDateTime::new(good_date, good_time)
								.and_local_timezone(Local)
								.unwrap();
							assignments.push(Assignment {
								due_date,
								name,
							});
						}
					});
			});
		
		Ok(Self {
			assignments_by_class,
			assign_dir: PathBuf::from(load_dir.as_ref()),
		})
	}

	pub fn save_to_file(&self) {
		if !self.assign_dir.try_exists().unwrap() {
			fs::create_dir(self.assign_dir.as_path()).unwrap();
		}

		for (class, assignments) in &self.assignments_by_class {
			let class_path = self.assign_dir.join(class);
			if !class_path.try_exists().unwrap() {
				fs::create_dir(class_path.as_path()).unwrap();
			}

			for assign in assignments {
				let assign_path = class_path.join(&assign.name);
				if !assign_path.try_exists().unwrap() {
					fs::create_dir(assign_path.as_path()).unwrap();
				}

				TodoList::write_str_to_file(assign_path.join("date"), assign.due_date.date_naive().format("%Y-%m-%d").to_string());
				TodoList::write_str_to_file(assign_path.join("time"), assign.due_date.time().format("%H:%M").to_string());
			}
		}
	}

	fn read_file_sans_newline<P>(file_path: P) -> String
		where P: AsRef<Path>
	{
		let file = File::open(file_path)
			.unwrap();
		let mut reader = BufReader::new(file);
		let mut file_content = String::new();
		let _len = reader.read_to_string(&mut file_content)
			.unwrap();

		if file_content.ends_with('\n') {
			file_content.pop();
		}

		file_content
	}

	fn write_str_to_file<P>(file_path: P, string: String)
		where P: AsRef<Path>
	{
		let file = File::create(file_path)
			.unwrap();
		let mut writer = BufWriter::new(file);
		let _len = writer.write(string.as_bytes()).unwrap();
	}
}

impl Drop for TodoList {
	fn drop(&mut self) {
		self.save_to_file();
	}
}

impl Clone for TodoList {
	fn clone(&self) -> Self {
		return Self {
			assignments_by_class: self.assignments_by_class.clone(),
			assign_dir: self.assign_dir.clone(),
		};
	}
}

impl PartialEq for TodoList {
	fn eq(&self, other: &Self) -> bool {
		return self.assignments_by_class == other.assignments_by_class &&
			self.assign_dir == other.assign_dir;
	}
}

impl Eq for TodoList {
}
