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
use std::fs::File;
use std::io::{prelude::*, BufReader, BufWriter};
use chrono::NaiveDate;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::assignment::Assignment;

pub struct TodoList {
	// class name to assignment ids
	uids_by_class: HashMap<String, Vec<u64>>,
	assignment_by_uid: HashMap<u64, Assignment>,
	list_path: PathBuf,
}

impl TodoList {
	pub fn new<P>(load_path: P) -> Result<Self,&'static str>
		where P: AsRef<Path>
	{
		if load_path.as_ref().exists() && load_path.as_ref().is_file() {
			let list_str = TodoList::read_file_sans_newline(&load_path);
			let assignments_by_class = serde_json::from_str::<BTreeMap<String, Vec<Assignment>>>(&list_str).unwrap();

			let mut uids_by_class = HashMap::new();
			let mut assignment_by_uid = HashMap::new();
			for (class, assignments) in assignments_by_class {
				uids_by_class.insert(class.clone(), vec![]);
				for assign in assignments {
					let uid = {
						let mut h = DefaultHasher::new();
						assign.hash(&mut h);
						h.finish()
					};
					assignment_by_uid.insert(uid, assign);
					uids_by_class.get_mut(&class).unwrap().push(uid);
				}
			}
			
			Ok(Self {
				uids_by_class,
				assignment_by_uid,
				list_path: PathBuf::from(load_path.as_ref()),
			})
		}
		else {
			Err("load_path isn't a proper file")
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
		//eprintln!("Saving todolist to file...");
		//if self.list_dir.try_exists().unwrap() {
			//fs::remove_dir_all(self.list_dir.as_path()).unwrap();
		//}

		let mut assignments_by_class = BTreeMap::<String, Vec<Assignment>>::new();
		for (class, uids) in &self.uids_by_class {
			assignments_by_class.insert(class.clone(), vec![]);
			for uid in uids {
				let assign = self.assignment_by_uid.get(&uid).unwrap();
				let assignments = assignments_by_class.get_mut(class).unwrap();
				assignments.push(assign.clone());
			}
		}

		let json = serde_json::to_string_pretty(&assignments_by_class).unwrap();
		//eprintln!("{}", &json);
		TodoList::write_str_to_file(&self.list_path, json);
		//eprintln!("Finish writing todolist...");
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
			list_path: self.list_path.clone(),
		}
	}
}

impl PartialEq for TodoList {
	fn eq(&self, other: &Self) -> bool {
		self.uids_by_class == other.uids_by_class &&
			self.assignment_by_uid == other.assignment_by_uid &&
			self.list_path == other.list_path
	}
}

impl Eq for TodoList {
}
