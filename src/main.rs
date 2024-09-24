use std::env;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::collections::HashMap;
use std::sync::Arc;
use std::vec::Vec;
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime};
use cursive::Cursive;
use cursive::views::{Button, Dialog, DummyView, EditView, TextView, LinearLayout, SelectView, Menubar, ScrollView, Panel};
use cursive::traits::*;

mod assignment;
mod todolist;

use assignment::Assignment;
use todolist::TodoList;

enum MainMessage {
	Quit,
	NewAssignment(String, Assignment),
	NewClass(String),
	GetClassAssignments(String),
	GetWeekAssignments(DateTime<Local>),
	GetClasses,
	CheckClassExists(String),
}

enum TodoMessage {
	NewAssignmentResponse(bool),
	SendClassAssignments(Vec<Assignment>),
	SendWeekAssignments(HashMap<String, Vec<Assignment>>),
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
		let res = match msg {
			MainMessage::Quit => break,
			MainMessage::NewClass(class) => {
				if !todolist.assignments_by_class.contains_key(&class) {
					todolist.assignments_by_class.insert(class, vec![]);
				}
				Ok(())
			},
			MainMessage::NewAssignment(class, assign) => {
				//if !todolist.assignments_by_class.contains_key(&class) {
				//	todolist.assignments_by_class.insert(class.clone(), vec![]);
				//}

				let ret = match todolist.assignments_by_class.get_mut(&class) {
					Some(class) => {
						class.push(assign);
						true
					},
					None => false,
				};

				send.send(TodoMessage::NewAssignmentResponse(ret))
			},
			MainMessage::GetClasses => {
				let mut classes = todolist.assignments_by_class.clone().into_keys().collect::<Vec<String>>();
				classes.sort();

				send.send(TodoMessage::SendClasses(classes))
			},
			MainMessage::GetClassAssignments(class) => {
				send.send(TodoMessage::SendClassAssignments(todolist.assignments_by_class.get(&class).unwrap().clone()))
			},
			MainMessage::GetWeekAssignments(from_date) => {
				let this_week = todolist.assignments_by_class
					.iter()
					.map(|(class, assignments)| {
						(class.clone(), assignments
							.clone()
							.iter()
							.filter(|assign| {
								let offset = (assign.due_date - from_date).num_seconds();
								offset >= 0 && offset < 60 * 60 * 24 * 7
							})
							.map(|assign| assign.clone())
							.collect())
					})
					.collect();

				send.send(TodoMessage::SendWeekAssignments(this_week))
			},
			MainMessage::CheckClassExists(class) => {
				send.send(TodoMessage::ClassExists(todolist.assignments_by_class.contains_key(&class)))
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

fn main() {
	let (send_main, receive_main) = mpsc::channel();
	let (send_todo, receive_todo) = mpsc::channel();
	let send_main2 = send_main.clone();

	let h = thread::spawn(move || todolist_thread(receive_main, send_todo));

	let mut siv = cursive::default();
	siv.set_user_data(CursiveData {
			sender: send_main,
			receiver: receive_todo,
		});

	let classes_view = {
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

		let classes_view = classes_view.on_submit(|s, name: &str| {
				select_class(s, Arc::new(name.to_string()))
			})
			.with_name("select")
			.min_size((15, 10));
		let classes_view = LinearLayout::vertical()
			.child(classes_view)
			.child(Button::new("Add new class", add_classname));

		Dialog::around(classes_view)
			.title("Classes")
	};

	let todo_str = {
		let (sender, receiver) = {
			let data = siv.user_data::<CursiveData>().unwrap();
			(&data.sender, &data.receiver)
		};
		get_todo_text(sender, receiver).unwrap()
	};
	let week_todo = Panel::new(TextView::new(todo_str).with_name("todolist"))
		.title("TODO This Week");

	let info_view = LinearLayout::horizontal()
		.child(classes_view)
		.child(DummyView)
		.child(week_todo);

	let buttons = LinearLayout::horizontal()
		.child(Button::new("Quit", Cursive::quit));

	siv.add_layer(Dialog::around(LinearLayout::vertical()
			.child(info_view)
			.child(DummyView)
			.child(buttons)));

	//let main_menu = Menubar::new()
		//.insert(

	siv.run();

	send_main2.clone().send(MainMessage::Quit).unwrap();
	h.join().unwrap();
}

fn get_todo_text(sender: &Sender<MainMessage>, receiver: &Receiver<TodoMessage>) -> Option<String> {
	sender.send(MainMessage::GetWeekAssignments(Local::now())).unwrap();
	if let TodoMessage::SendWeekAssignments(w) = receiver.recv().unwrap() {
		Some(w.iter()
			.map(|(_class, assignments)| {
				assignments.iter()
					.map(|assign| {
						let trunc_name = assign.name.chars().into_iter().take(24).collect::<String>();
						String::from(format!("{:<24} {}\n", trunc_name, assign.due_date.format("Due %B %e, %l:%M %p")))
					})
					.fold(String::new(), |prev, s| prev + &s)
			})
			.fold(String::new(), |prev, s| prev + &s))
	}
	else {
		None
	}
}

fn get_assign_text(classname: String, sender: &Sender<MainMessage>, receiver: &Receiver<TodoMessage>) -> Option<String> {
	sender.send(MainMessage::GetClassAssignments(classname)).unwrap();
	if let TodoMessage::SendClassAssignments(assignments) = receiver.recv().unwrap() {
		let now = Local::now();

		Some(assignments.iter()
			.filter_map(|assign| {
				let offset = (assign.due_date - now).num_seconds();

				// only write if the deadline hasn't passed, or if it's not more than a week away from today
				if offset >= 0 && offset < 60 * 60 * 24 * 7 {
					Some(format!("{:<32} {}\n",
							assign.name,
							assign.due_date.format("Due %B %e, %l:%M %p"))
						.to_string())
				}
				else {
					None
				}
			})
			.fold(String::new(), |prev, s| prev + &s))
	}
	else {
		None
	}
}

fn select_class(s: &mut Cursive, name: Arc<String>) {
	let (sender, receiver) = {
		let data = s.user_data::<CursiveData>().unwrap();
		(&data.sender, &data.receiver)
	};

	let list = get_assign_text((*name).clone(), sender, receiver).unwrap();
	let text_view = TextView::new(list)
		.with_name("assigns");
	s.add_layer(Dialog::around(ScrollView::new(text_view))
		.button("Add new assignment", move |s| {
			add_assignment(s, name.clone());
		})
		.button("OK", |s| {
			s.pop_layer();
		}));
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

fn add_assignment(s: &mut Cursive, classname: Arc<String>) {
	let name = EditView::new()
		.with_name("new_name")
		.fixed_width(20);
	let date = EditView::new()
		.with_name("date")
		.fixed_width(11);
	let time = EditView::new()
		.with_name("time")
		.fixed_width(6);
	s.add_layer(Dialog::around(LinearLayout::vertical()
			.child(name)
			.child(date)
			.child(time))
		.title("Enter a new assignment")
		.button("OK", move |s| {
			let name = s.call_on_name("new_name", |view: &mut EditView| {
				view.get_content()
			}).unwrap();
			let date_str = s.call_on_name("date", |view: &mut EditView| {
				view.get_content()
			}).unwrap();
			let time_str = s.call_on_name("time", |view: &mut EditView| {
				view.get_content()
			}).unwrap();

			let (sender, receiver) = {
				let data = s.user_data::<CursiveData>().unwrap();
				(&data.sender, &data.receiver)
			};

			let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
			let time = NaiveTime::parse_from_str(&time_str, "%H:%M").ok();
			let res = if let (Some(good_date), Some(good_time)) = (date, time) {
				let due_date = NaiveDateTime::new(good_date, good_time)
					.and_local_timezone(Local)
					.unwrap();
				sender.send(MainMessage::NewAssignment(classname.to_string(), Assignment {
						due_date,
						name: (*name).clone(),
					}))
				.unwrap();


				if let TodoMessage::NewAssignmentResponse(res) = receiver.recv().unwrap() {
					res
				}
				else {
					false
				}
			}
			else {
				let dialog = Dialog::around(TextView::new("Formating error with date/time")).button("Ok", Cursive::noop);
				s.add_layer(dialog);
				s.pop_layer();
				false
			};

			if !res {
				let dialog = Dialog::around(TextView::new("Failed to add new assignment successfully")).button("Ok", Cursive::noop);
				s.add_layer(dialog);
				s.pop_layer();
			}

			s.pop_layer();

			let (todo_text, assign_text) = {
				let (sender, receiver) = {
					let data = s.user_data::<CursiveData>().unwrap();
					(&data.sender, &data.receiver)
				};

				let todo_list = get_todo_text(sender, receiver).unwrap();
				let assign_text = get_assign_text((*classname).clone(), sender, receiver).unwrap();
				(todo_list, assign_text)
			};
			s.call_on_name("todolist", move |list: &mut TextView| {
				list.set_content(todo_text);
			});
			s.call_on_name("assigns", move |list: &mut TextView| {
				list.set_content(assign_text);
			});
		})
		.button("Cancel", |s| {
			s.pop_layer();
		}));
}
