/*
Copyright (C) 2024 Ryan Rhee

This program is free software; you can redistribute it and/or
modify it under the terms of the GNU General Public License
as published by the Free Software Foundation; either version 2
of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program; if not, see
<https://www.gnu.org/licenses/>.
*/

use std::vec::Vec;
use std::collections::{HashMap,BTreeMap};
use std::path::{Path, PathBuf};
use std::fs::{self,File};
use std::io::{prelude::*, BufReader, BufWriter};
use chrono::{NaiveDate, NaiveTime, NaiveDateTime, Local};
use chrono::offset::MappedLocalTime;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::process::Command;
use serde::{Deserialize, Serialize};
use log;

use crate::assignment::Assignment;
use crate::assignment::AssignmentV1;

pub struct TodoList {
	// class name to assignment ids
	uids_by_class: HashMap<String, Vec<u64>>,
	assignment_by_uid: HashMap<u64, Assignment>,
	completed_by_uid: HashMap<u64, bool>,
	ghost_uids: Vec<u64>,
	list_path: PathBuf,
}

type TodoListV1 = BTreeMap<String, Vec<AssignmentV1>>;
#[derive(Serialize, Deserialize)]
struct TodoListV2 {
	version: u8,
	assignments: BTreeMap<String, Vec<(Assignment, bool)>>,
}

struct TodoListParsed {
	uids_by_class: HashMap<String, Vec<u64>>,
	assignment_by_uid: HashMap<u64, Assignment>,
	completed_by_uid: HashMap<u64, bool>,
}

impl TodoList {
	pub fn new<P>(load_path: P, script_path: P) -> Result<Self,&'static str>
		where P: AsRef<Path>
	{
		if load_path.as_ref().exists() && load_path.as_ref().is_file() {
			let list_str = TodoList::read_file_sans_newline(&load_path);
			let parsed;
			if let Ok(v1) = Self::parse_v1(&list_str) {
				parsed = v1;
			}
			else if let Ok(v2) = Self::parse_v2(&list_str) {
				parsed = v2;
			}
			else {
				return Err("Failed to parse log file, pretending it's blank");
			}

			let mut uids_by_class = parsed.uids_by_class;
			let mut assignment_by_uid = parsed.assignment_by_uid;
			let mut completed_by_uid = parsed.completed_by_uid;
			let mut ghost_uids = Vec::new();

			if script_path.as_ref().exists() && script_path.as_ref().is_dir() {
				fs::read_dir(script_path)
					.unwrap()
					.for_each(|entry| {
						let entry = entry.unwrap();

						if !entry.file_type().unwrap().is_file() {
							return;
						}

						if let Ok(output) = Command::new(entry.path()).output() {
							if let Ok(lines) = String::from_utf8(output.stdout) {
								lines.lines()
									.for_each(|line| {
										let tokens: Vec<&str> = line.split(",")
											.collect();

										if tokens.len() != 4 {
											return;
										}

										if let (Ok(date), Ok(time)) = (NaiveDate::parse_from_str(tokens[2], "%Y-%m-%d"), NaiveTime::parse_from_str(tokens[3], "%H:%M"))
										{
											if let MappedLocalTime::Single(due_date) = NaiveDateTime::new(date, time).and_local_timezone(Local) {
												let classname = tokens[0];
												let name = tokens[1];
												let assign = Assignment{
													due_date,
													name: name.to_string(),
												};
												let uid = {
													let mut h = DefaultHasher::new();
													assign.hash(&mut h);
													h.finish()
												};
												if assignment_by_uid.contains_key(&uid) {
													return;
												}
												assignment_by_uid.insert(uid, assign);
												completed_by_uid.insert(uid, false);
												ghost_uids.push(uid);
												if let Some(uids) = uids_by_class.get_mut(classname) {
													uids.push(uid);
												}
											}
										}
									});
							}
						}

					});
			}

			Ok(Self {
				uids_by_class,
				assignment_by_uid,
				completed_by_uid,
				ghost_uids,
				list_path: PathBuf::from(load_path.as_ref()),
			})
		}
		else {
			log::info!("Couldn't read todolist at {}, creating blank one", load_path.as_ref().display());
			let _ = File::create_new(&load_path);
			Ok(Self {
				uids_by_class: HashMap::new(),
				assignment_by_uid: HashMap::new(),
				completed_by_uid: HashMap::new(),
				ghost_uids: Vec::new(),
				list_path: PathBuf::from(load_path.as_ref()),
			})
		}
	}

	fn parse_v1(list_str: &String) -> Result<TodoListParsed, &'static str> {
		if let Ok(assignments_by_class) = serde_json::from_str::<TodoListV1>(&list_str) {
			let mut uids_by_class = HashMap::new();
			let mut assignment_by_uid = HashMap::new();
			let mut completed_by_uid = HashMap::new();
			for (class, assignments) in assignments_by_class {
				uids_by_class.insert(class.clone(), vec![]);
				for assign in assignments {
					let uid = {
						let mut h = DefaultHasher::new();
						assign.hash(&mut h);
						h.finish()
					};
					let completed = assign.completed;
					let assign = Assignment{
						due_date: assign.due_date,
						name: assign.name,
					};
					assignment_by_uid.insert(uid, assign);
					completed_by_uid.insert(uid, completed);
					uids_by_class.get_mut(&class).unwrap().push(uid);
				}
			}

			Ok(TodoListParsed{
				uids_by_class,
				assignment_by_uid,
				completed_by_uid,})
		}
		else {
			Err("Failed to read file as V1")
		}
	}

	fn parse_v2(list_str: &String) -> Result<TodoListParsed,&'static str> {
		if let Ok(todo_list_file) = serde_json::from_str::<TodoListV2>(&list_str) {
			let mut uids_by_class = HashMap::new();
			let mut assignment_by_uid = HashMap::new();
			let mut completed_by_uid = HashMap::new();
			for (class, assignments) in todo_list_file.assignments {
				uids_by_class.insert(class.clone(), vec![]);
				for (assign, completed) in assignments {
					let uid = {
						let mut h = DefaultHasher::new();
						assign.hash(&mut h);
						h.finish()
					};
					assignment_by_uid.insert(uid, assign);
					completed_by_uid.insert(uid, completed);
					uids_by_class.get_mut(&class).unwrap().push(uid);
				}
			}

			Ok(TodoListParsed{
				uids_by_class,
				assignment_by_uid,
				completed_by_uid,})
		}
		else {
			Err("Failed to read file as V2")
		}
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
					self.completed_by_uid.insert(uid, false);
					Ok(uid)
				}
			},
			None => Err(()),
		}
	}

	pub fn get_classes(&self) -> Vec<String> {
		let mut classes: Vec<String> = self.uids_by_class.iter()
			.map(|(class, _uids)| class.clone())
			.collect();
		classes.sort();
		classes
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
		match self.completed_by_uid.get_mut(&uid) {
			Some(comp) => {
				*comp = completed;
				Ok(())
			},
			None => Err(()),
		}
	}

	pub fn get_assignment_completion(&self, uid: u64) -> Result<bool, ()> {
		match self.completed_by_uid.get(&uid) {
			Some (comp) => {
				Ok(*comp)
			},
			None => Err(()),
		}
	}

	pub fn save_to_file(&self) {
		log::info!("Saving todolist to file...");
		//if self.list_dir.try_exists().unwrap() {
			//fs::remove_dir_all(self.list_dir.as_path()).unwrap();
		//}

		let mut serialize = TodoListV2{
			version: 2,
			assignments: BTreeMap::<_, _>::new(),
		};
		for (class, uids) in &self.uids_by_class {
			let mut assignments = vec![];
			for uid in uids {
				let assign = self.assignment_by_uid.get(&uid).unwrap();
				let completed = *self.completed_by_uid.get(&uid).unwrap();
				let is_ghost = self.ghost_uids.contains(uid);
				if !is_ghost || (is_ghost && completed) {
					assignments.push((assign.clone(), completed));
				}
			}
			serialize.assignments.insert(class.clone(), assignments);
		}

		let json = serde_json::to_string_pretty(&serialize).unwrap();
		TodoList::write_str_to_file(&self.list_path, json);
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
		if !string.ends_with('\n') {
			let _len = writer.write("\n".as_bytes()).unwrap();
		}
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
			completed_by_uid: self.completed_by_uid.clone(),
			ghost_uids: self.ghost_uids.clone(),
			list_path: self.list_path.clone(),
		}
	}
}

impl PartialEq for TodoList {
	fn eq(&self, other: &Self) -> bool {
		self.uids_by_class == other.uids_by_class &&
			self.assignment_by_uid == other.assignment_by_uid &&
			self.completed_by_uid == other.completed_by_uid &&
			self.ghost_uids == other.ghost_uids &&
			self.list_path == other.list_path
	}
}

impl Eq for TodoList {
}
