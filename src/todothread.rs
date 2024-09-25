use std::thread::{self, JoinHandle};
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender, Receiver};
use std::path::{Path, PathBuf};
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime};

use crate::assignment::Assignment;
use crate::todolist::TodoList;

enum MainMessage {
	Quit,
	NewAssignment(String, Assignment),
	NewClass(String),
	GetClassAssignments(String),
	GetWeekAssignments(NaiveDate),
	GetClasses,
	CheckClassExists(String),
}

enum TodoMessage {
	NewAssignmentResponse(bool),
	NewClassResponse(bool),
	SendClassAssignments(Vec<Assignment>),
	SendWeekAssignments(HashMap<String, Vec<Assignment>>),
	SendClasses(Vec<String>),
	ClassExists(bool),
}

struct TodoThreadInternal {
	list: TodoList,
	main_recv: Receiver<MainMessage>,
	todo_send: Sender<TodoMessage>,
}

impl TodoThreadInternal {
	fn new<P>(todo_dir_path: P, main_recv: Receiver<MainMessage>, todo_send: Sender<TodoMessage>) -> Self
		where P: AsRef<Path>
	{
		let list = TodoList::new(todo_dir_path).unwrap();
		Self {
			list,
			main_recv,
			todo_send
		}
	}

	fn msg_loop(&mut self) {
		while let Ok(msg) = self.main_recv.recv() {
			let res = match msg {
				MainMessage::Quit => break,
				MainMessage::NewClass(class) => {
					let ret = if !self.list.assignments_by_class.contains_key(&class) {
						self.list.assignments_by_class.insert(class, vec![]);
						true
					}
					else {
						false
					};

					self.todo_send.send(TodoMessage::NewClassResponse(ret))
				},
				MainMessage::NewAssignment(class, assign) => {
					//if !todolist.assignments_by_class.contains_key(&class) {
					//	todolist.assignments_by_class.insert(class.clone(), vec![]);
					//}

					let ret = match self.list.assignments_by_class.get_mut(&class) {
						Some(class) => {
							class.push(assign);
							true
						},
						None => false,
					};

					self.todo_send.send(TodoMessage::NewAssignmentResponse(ret))
				},
				MainMessage::GetClasses => {
					let mut classes = self.list.assignments_by_class.clone().into_keys().collect::<Vec<String>>();
					classes.sort();

					self.todo_send.send(TodoMessage::SendClasses(classes))
				},
				MainMessage::GetClassAssignments(class) => {
					self.todo_send.send(TodoMessage::SendClassAssignments(self.list.assignments_by_class.get(&class).unwrap().clone()))
				},
				MainMessage::GetWeekAssignments(from_date) => {
					let this_week = self.list.assignments_by_class
						.iter()
						.map(|(class, assignments)| {
							(class.clone(), assignments
								.clone()
								.iter()
								.filter(|assign| {
									let offset = (assign.due_date.date_naive() - from_date).num_seconds();
									offset >= 0 && offset < 60 * 60 * 24 * 7
								})
								.map(|assign| assign.clone())
								.collect())
						})
						.collect();

					self.todo_send.send(TodoMessage::SendWeekAssignments(this_week))
				},
				MainMessage::CheckClassExists(class) => {
					self.todo_send.send(TodoMessage::ClassExists(self.list.assignments_by_class.contains_key(&class)))
				},
			};

			match res {
				Ok(_) => (),
				Err(_) => break,
			}
		}

		eprintln!("Exiting todolist thread");
		/*let now = Local::now();
		for (class, assignments) in &todolist.assignments_by_class {
			println!("{}", class);
			for assign in assignments {
				let offset = (assign.due_date - now).num_seconds();
				// only write if the deadline hasn't passed, or if it's not more than a week away from today
				if offset >= 0 && offset < 60 * 60 * 24 * 7 {
					println!("{:<20} {}", assign.name, assign.due_date.format("Due %B %e, %l:%M %p"));
				}
			}
		}*/
	}
}

pub struct TodoThread {
	main_send: Sender<MainMessage>,
	todo_recv: Receiver<TodoMessage>,
	handle: Option<JoinHandle<()>>,
}

impl TodoThread {
	pub fn new<P>(todo_dir_path: P) -> Self
		where P: AsRef<Path>
	{
		let (main_send, main_recv) = mpsc::channel();
		let (todo_send, todo_recv) = mpsc::channel();
		let mut todo_internal = TodoThreadInternal::new(todo_dir_path, main_recv, todo_send);
		let handle = Some(thread::spawn(move || todo_internal.msg_loop()));

		Self {
			main_send,
			todo_recv,
			handle,
		}
	}

	pub fn quit(&self) -> Result<(), ()> {
		match self.main_send.send(MainMessage::Quit) {
			Ok(_) => Ok(()),
			Err(_) => Err(()),
		}
	}
	
	pub fn new_assignment(&self, classname: String, assignment: Assignment) -> Result<bool, ()> {
		self.main_send.send(MainMessage::NewAssignment(classname, assignment)).unwrap();
		if let TodoMessage::NewAssignmentResponse(b) = self.todo_recv.recv().unwrap() {
			Ok(b)
		}
		else {
			Err(())
		}
	}

	pub fn new_class(&self, classname: String) -> Result<bool, ()> {
		self.main_send.send(MainMessage::NewClass(classname)).unwrap();
		if let TodoMessage::NewClassResponse(b) = self.todo_recv.recv().unwrap() {
			Ok(b)
		}
		else {
			Err(())
		}
	}

	pub fn get_class_assignments(&self, classname: String) -> Result<Vec<Assignment>, ()> {
		self.main_send.send(MainMessage::GetClassAssignments(classname)).unwrap();
		if let TodoMessage::SendClassAssignments(a) = self.todo_recv.recv().unwrap() {
			Ok(a)
		}
		else {
			Err(())
		}
	}

	pub fn get_week_assignments(&self, curr_time: NaiveDate) -> Result<HashMap<String, Vec<Assignment>>, ()> {
		self.main_send.send(MainMessage::GetWeekAssignments(curr_time)).unwrap();
		if let TodoMessage::SendWeekAssignments(a) = self.todo_recv.recv().unwrap() {
			Ok(a)
		}
		else {
			Err(())
		}
	}

	pub fn get_classes(&self) -> Result<Vec<String>, ()> {
		self.main_send.send(MainMessage::GetClasses).unwrap();
		if let TodoMessage::SendClasses(a) = self.todo_recv.recv().unwrap() {
			Ok(a)
		}
		else {
			Err(())
		}
	}

	pub fn check_class_exists(&self, classname: String) -> Result<bool, ()> {
		self.main_send.send(MainMessage::CheckClassExists(classname)).unwrap();
		if let TodoMessage::ClassExists(a) = self.todo_recv.recv().unwrap() {
			Ok(a)
		}
		else {
			Err(())
		}
	}
}

impl Drop for TodoThread {
	fn drop(&mut self) {
		let _ = self.quit();
		if let Some(handle) = self.handle.take() {
			let _ = handle.join();
		}
	}
}
