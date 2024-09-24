use std::env;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::collections::HashMap;
use std::vec::Vec;
use chrono::prelude::*;
use cursive::Cursive;
use cursive::views::{Button, Dialog, DummyView, EditView, TextView, LinearLayout, SelectView, Menubar, ScrollView};
use cursive::traits::*;

mod assignment;
mod todolist;

use assignment::Assignment;
use todolist::TodoList;

enum MainMessage {
	Stop,
	NewAssignment(String, Assignment),
	NewClass(String),
	GetClassAssignments(String),
	GetClasses,
	CheckClassExists(String),
}

enum TodoMessage {
	SendClassAssignments(Vec<Assignment>),
	SendClasses(Vec<String>),
	ClassExists(bool),
}

struct CursiveData {
	sender: Sender<MainMessage>,
	receiver: Receiver<TodoMessage>,
}

fn todolist_thread(receive: Receiver<MainMessage>, send: Sender<TodoMessage>) {
	let listpath = env::var("HOME").unwrap() + "/.todolist";
	let mut todolist = TodoList::new(listpath).unwrap();

	while let Ok(msg) = receive.recv() {
		match msg {
			MainMessage::Stop => break,
			MainMessage::NewClass(class) => {
				if !todolist.assignments_by_class.contains_key(&class) {
					todolist.assignments_by_class.insert(class, vec![]);
				}
			},
			MainMessage::NewAssignment(class, assign) => {
				//if !todolist.assignments_by_class.contains_key(&class) {
				//	todolist.assignments_by_class.insert(class.clone(), vec![]);
				//}

				todolist.assignments_by_class.get_mut(&class).unwrap().push(assign);
			},
			MainMessage::GetClasses => {
				let mut classes = todolist.assignments_by_class.clone().into_keys().collect::<Vec<String>>();
				classes.sort();

				match send.send(TodoMessage::SendClasses(classes)) {
					Ok(_) => (),
					Err(_) => break,
				}
			},
			MainMessage::GetClassAssignments(class) => {
				match send.send(TodoMessage::SendClassAssignments(todolist.assignments_by_class.get(&class).unwrap().clone())) {
					Ok(_) => (),
					Err(_) => break,
				}
			},
			MainMessage::CheckClassExists(class) => {
				match send.send(TodoMessage::ClassExists(todolist.assignments_by_class.contains_key(&class))) {
					Ok(_) => (),
					Err(_) => break,
				}
			},
		}
	}

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

fn main() {
	let (send_main, receive_main) = mpsc::channel();
	let (send_todo, receive_todo) = mpsc::channel();

	thread::spawn(move || todolist_thread(receive_main, send_todo));

	let mut siv = cursive::default();
	siv.set_user_data(CursiveData {
			sender: send_main,
			receiver: receive_todo,
		});

	let mut classes_view = SelectView::<String>::new();
	{
		let (sender, receiver) = {
			let data = siv.user_data::<CursiveData>().unwrap();
			(&data.sender, &data.receiver)
		};

		sender.send(MainMessage::GetClasses).unwrap();
		if let TodoMessage::SendClasses(c) = receiver.recv().unwrap() {
			classes_view.add_all_str(c);
		}
	}
	let classes_view = classes_view.on_submit(select_class)
		.with_name("select")
		.min_size((60, 40));

	let buttons = LinearLayout::vertical()
		.child(Button::new("Add new class", add_classname))
		.child(Button::new("Quit", Cursive::quit));

	siv.add_layer(Dialog::around(LinearLayout::vertical()
			.child(classes_view)
			.child(DummyView)
			.child(buttons))
		.title("Select a class"));

	//let main_menu = Menubar::new()
		//.insert(

	siv.run();
}

fn select_class(s: &mut Cursive, name: &str) {
	let (sender, receiver) = {
		let data = s.user_data::<CursiveData>().unwrap();
		(&data.sender, &data.receiver)
	};
	sender.send(MainMessage::GetClassAssignments(name.to_string())).unwrap();
	if let TodoMessage::SendClassAssignments(assignments) = receiver.recv().unwrap() {
		let mut text_view = TextView::new("");
		let now = Local::now();

		for assign in assignments {
			let offset = (assign.due_date - now).num_seconds();

			// only write if the deadline hasn't passed, or if it's not more than a week away from today
			if offset >= 0 && offset < 60 * 60 * 24 * 7 {
				text_view.append(format!("{:<20} {}\n", assign.name, assign.due_date.format("Due %B %e, %l:%M %p")));
			}
		}
		s.add_layer(Dialog::around(ScrollView::new(text_view))
			.button("OK", |s| {
				s.pop_layer();
			}));
	}
}

fn add_classname(s: &mut Cursive) {
	fn ok(s: &mut Cursive, name: &str) {
		{
			let (sender, receiver) = {
				let data = s.user_data::<CursiveData>().unwrap();
				(&data.sender, &data.receiver)
			};

			sender.send(MainMessage::CheckClassExists(name.to_string())).unwrap();
			if let TodoMessage::ClassExists(exists) = receiver.recv().unwrap() {
				if exists {
					s.pop_layer();
					return;
				}
			}
		}

		s.call_on_name("select", |view: &mut SelectView<String>| {
			view.add_item_str(name);
		});
		s.user_data::<CursiveData>()
			.unwrap()
			.sender.send(MainMessage::NewClass(name.to_string()))
			.unwrap();
		s.pop_layer();
	}

	s.add_layer(Dialog::around(EditView::new()
			.on_submit(ok)
			.with_name("name")
			.fixed_width(10))
		.title("Enter a new class name")
		.button("OK", |s| {
			let name = s.call_on_name("name", |view: &mut EditView| {
				view.get_content()
			}).unwrap();
			ok(s, &name);
		})
		.button("Cancel", |s| {
			s.pop_layer();
		}));
}
