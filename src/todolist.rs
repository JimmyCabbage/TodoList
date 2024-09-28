use std::vec::Vec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, BufWriter};
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::assignment::Assignment;

pub struct TodoList {
	// class name to assignment ids
	uids_by_class: HashMap<String, Vec<u64>>,
	assignment_by_uid: HashMap<u64, Assignment>,
	assign_dir: PathBuf,
}

impl TodoList {
	pub fn new<P>(load_dir: P) -> Result<Self,&'static str>
		where P: AsRef<Path>
	{
		if !load_dir.as_ref().is_dir() {
			return Err("load_dir is not a directory");
		}

		let mut uids_by_class = HashMap::new();
		let mut assignment_by_uid = HashMap::new();

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
				uids_by_class.insert(classname.clone(), vec![]);
				let assignments = uids_by_class.get_mut(&classname).unwrap();

				fs::read_dir(e.path())
					.unwrap()
					.filter_map(|e| e.ok())
					.filter(|e| e.file_type().unwrap().is_dir())
					.for_each(|e| {
						let name = e.file_name().as_os_str().to_str().unwrap().to_string();
						let mut date = None;
						let mut time = None;
						let mut completed = false;
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
									},
									"time" => {
										let file_data = TodoList::read_file_sans_newline(path);
										time = NaiveTime::parse_from_str(&file_data, "%H:%M").ok();
									},
									"complete" => {
										completed = true;
									},
									_ => (),
								};
							});

						if let (Some(good_date), Some(good_time)) = (date, time) {
							let due_date = NaiveDateTime::new(good_date, good_time)
								.and_local_timezone(Local)
								.unwrap();

							let assign = Assignment {
								due_date,
								name,
								completed,
							};
							let uid = {
								let mut h = DefaultHasher::new();
								assign.hash(&mut h);
								h.finish()
							};

							assignments.push(uid);
							assignment_by_uid.insert(uid, assign);
						}
					});
			});
		
		Ok(Self {
			uids_by_class,
			assignment_by_uid,
			assign_dir: PathBuf::from(load_dir.as_ref()),
		})
	}

	pub fn create_class(&mut self, classname: String) -> Result<(), ()> {
		if !self.uids_by_class.contains_key(&classname) {
			self.uids_by_class.insert(classname, vec![]);
			Ok(())
		}
		else {
			Err(())
		}
	}

	pub fn delete_class(&mut self, classname: String) -> Result<(), ()> {
		if self.uids_by_class.contains_key(&classname) {
			let uids = self.uids_by_class.get(&classname).unwrap();

			// check if other classes have this assign, else remove
			for uid in uids {
				let remaining = self.uids_by_class.get(&classname).unwrap()
					.iter()
					.find(|u| *u == uid)
					.is_some();
				if !remaining {
					self.assignment_by_uid.remove(uid);
				}
			}

			self.uids_by_class.remove(&classname);
			Ok(())
		}
		else {
			Err(())
		}
	}

	pub fn create_assignment(&mut self, classname: String, assignment: Assignment) -> Result<u64, ()> {
		match self.uids_by_class.get_mut(&classname) {
			Some(class) => {
				let uid = {
					let mut h = DefaultHasher::new();
					assignment.hash(&mut h);
					h.finish()
				};

				if self.assignment_by_uid.contains_key(&uid) {
					Err(())
				}
				else {
					class.push(uid);
					self.assignment_by_uid.insert(uid, assignment);
					Ok(uid)
				}
			},
			None => Err(()),
		}
	}

	pub fn get_classes(&self) -> Vec<String> {
		self.uids_by_class.iter()
			.map(|(class, _uids)| class.clone())
			.collect()
	}

	pub fn get_class_assignments(&self, classname: &String) -> Result<Vec<Assignment>, ()> {
		let uids = self.uids_by_class.get(classname);
		if uids.is_none() {
			Err(())
		}
		else {
			Ok(uids.unwrap().iter()
				.map(|uid| {
					self.assignment_by_uid.get(uid).unwrap().clone()
				}).collect())
		}
	}

	pub fn get_timespan_assignments(&self, start_date: NaiveDate, end_date: NaiveDate) -> HashMap<String, Vec<Assignment>> {
		self.uids_by_class
			.iter()
			.map(|(class, uids)| {
				(class.clone(), uids
					.clone()
					.iter()
					.map(|uid| self.assignment_by_uid.get(uid).unwrap())
					.filter(|assign| {
						let naive = assign.due_date.date_naive();
						start_date <= naive &&
							end_date >= naive
						//let offset = (assign.due_date.date_naive() - from_date).num_seconds();
						//offset >= 0 && offset < 60 * 60 * 24 * 7
						// also include 2 day old assignments
					}).map(|assign| assign.clone())
					.collect())
			}).collect()
	}

	pub fn set_assignment_completion(&mut self, uid: u64, completed: bool) -> Result<(), ()> {
		match self.assignment_by_uid.get_mut(&uid) {
			Some (assign) => {
				assign.completed = completed;
				Ok(())
			},
			None => Err(()),
		}
	}

	pub fn get_assignment_completion(&self, uid: u64) -> Result<bool, ()> {
		match self.assignment_by_uid.get(&uid) {
			Some (assign) => {
				Ok(assign.completed)
			},
			None => Err(()),
		}
	}

	pub fn save_to_file(&self) {
		eprintln!("Saving todolist to file...");
		if !self.assign_dir.try_exists().unwrap() {
			fs::create_dir(self.assign_dir.as_path()).unwrap();
		}

		for entry in fs::read_dir(&self.assign_dir.as_path()).unwrap() {
			match entry {
				Ok(entry) => {
					let file_type = entry.file_type().unwrap();
					if file_type.is_dir() {
						std::fs::remove_dir_all(entry.path().as_path()).unwrap();
					}
					else if file_type.is_file() {
						std::fs::remove_file(entry.path().as_path()).unwrap();
					}
				},
				Err(_) => (),
			}
		}

		for (class, uids) in &self.uids_by_class {
			let class_path = self.assign_dir.join(class);
			if !class_path.try_exists().unwrap() {
				fs::create_dir(class_path.as_path()).unwrap();
			}

			for uid in uids {
				let assign = self.assignment_by_uid.get(uid).unwrap();
				let assign_path = class_path.join(&assign.name);
				if !assign_path.try_exists().unwrap() {
					fs::create_dir(assign_path.as_path()).unwrap();
				}

				TodoList::write_str_to_file(assign_path.join("date"), assign.due_date.date_naive().format("%Y-%m-%d").to_string());
				TodoList::write_str_to_file(assign_path.join("time"), assign.due_date.time().format("%H:%M").to_string());
				if assign.completed {
					TodoList::write_str_to_file(assign_path.join("complete"), String::new());
				}
			}
		}
		eprintln!("Finish writing todolist...");
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
		Self {
			uids_by_class: self.uids_by_class.clone(),
			assignment_by_uid: self.assignment_by_uid.clone(),
			assign_dir: self.assign_dir.clone(),
		}
	}
}

impl PartialEq for TodoList {
	fn eq(&self, other: &Self) -> bool {
		self.uids_by_class == other.uids_by_class &&
			self.assignment_by_uid == other.assignment_by_uid &&
			self.assign_dir == other.assign_dir
	}
}

impl Eq for TodoList {
}
