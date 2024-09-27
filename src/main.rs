use std::env;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;
use std::vec::Vec;
use std::hash::{DefaultHasher, Hash, Hasher};
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime, Days};
use cursive::Cursive;
use cursive::views::{Button, Dialog, DummyView, EditView, TextView, LinearLayout, SelectView, Menubar, ScrollView, Panel, Checkbox, NamedView, ListView};
use cursive::traits::*;

mod assignment;
mod todolist;

use assignment::Assignment;
use todolist::TodoList;

fn main() {
	let listpath = env::var("HOME").unwrap() + "/.todolist";
	let todolist = RefCell::new(TodoList::new(listpath).unwrap());

	let mut siv = cursive::default();
	siv.set_user_data(todolist.clone());

	let classes_view = {
		let mut classes_view = SelectView::<String>::new();
		classes_view.add_all_str(todolist.borrow().get_classes());

		let classes_view = classes_view.on_submit(|s, name: &str| {
				select_class(s, Arc::new(name.to_string()))
			}).with_name("select")
			.min_size((15, 10));

		let classes_view = LinearLayout::vertical()
			.child(classes_view)
			.child(Button::new("Add new class", add_classname));

		Dialog::around(classes_view)
			.title("Classes")
	};

	let week_todo = {
		let mut vert = LinearLayout::vertical().with_name("weektodo");
		make_todo_list(&todolist.borrow(), &mut (*vert.get_mut()));
		let vert = ScrollView::new(vert);
		Dialog::around(vert)
			.title("TODO This Week")
	};

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
}

fn make_todo_list(todolist: &TodoList, vert: &mut LinearLayout) {
	let assignments_by_date = {
		let now = Local::now().date_naive();
		let all_week_assignments = {
			let begin = now.checked_sub_days(Days::new(3)).unwrap();
			let end = now.checked_add_days(Days::new(7)).unwrap();
			todolist.get_timespan_assignments(begin, end)
		};

		let mut date_assign = HashMap::new();
		for i in -3i64..=7 {
			let date = if i > 0 {
				now.checked_add_days(Days::new(i as u64)).unwrap()
			}
			else if i < 0 {
				now.checked_sub_days(Days::new(-i as u64)).unwrap()
			}
			else {
				now
			};

			let mut date_assigns = vec![];
			for (class, assignments) in &all_week_assignments {
				for assign in assignments {
					if assign.due_date.date_naive() == date {
						date_assigns.push((class.clone(), assign.clone()));
					}
				}
			}

			if !date_assigns.is_empty() {
				date_assign.insert(date.clone(), date_assigns);
			}
		}

		date_assign
	};

	vert.clear();
	let mut dates = assignments_by_date.keys().collect::<Vec<&NaiveDate>>();
	dates.sort();
	for date in dates {
		let assignments = {
			let mut assignments = assignments_by_date.get(date).unwrap().clone();
			assignments.sort_by(|(_, a), (_, b)| {
				a.due_date.cmp(&b.due_date)
			});
			assignments
		};
		vert.add_child(TextView::new(date.format("Due %B %e").to_string()));

		let time_format_str = "%l:%M %p";
		let max_assign_name_len = 32;
		vert.add_child(TextView::new("─".repeat(4) + "┬" + &"─".repeat(time_format_str.len() + 2) + "┬" + &"─".repeat(max_assign_name_len + 1)));
		for (_class, assign) in assignments {
			let due_date = assign.due_date.format(time_format_str).to_string();
			let uid = {
				let mut h = DefaultHasher::new();
				assign.hash(&mut h);
				h.finish()
			};
			let check = Checkbox::new().with_checked(todolist.get_assignment_completion(uid).unwrap())
				.on_change(move |s, checked| {
					let mut todolist = s.user_data::<RefCell<TodoList>>().unwrap().borrow_mut();
					todolist.set_assignment_completion(uid, checked).unwrap();
				});
			vert.add_child(LinearLayout::horizontal()
				.child(check)
				.child(TextView::new(" │ "))
				.child(TextView::new(due_date))
				.child(TextView::new(" │ "))
				.child(ScrollView::new(TextView::new(assign.name.clone())).max_width(max_assign_name_len)));
		}
		vert.add_child(DummyView);
	}
					/*.map(|assign| {
						let trunc_name = assign.name.chars().into_iter().take(24).collect::<String>();
						String::from(format!("{:<24} {}\n", trunc_name, assign.due_date.format("Due %B %e, %l:%M %p")))
					})
					.fold(String::new(), |prev, s| prev + &s)*/
}

fn get_assign_text(todolist: &TodoList, classname: String) -> String {
	let assignments = todolist.get_class_assignments(&classname).unwrap();
	let now = Local::now().date_naive();

	assignments.iter()
		.filter_map(|assign| {
			let offset = (assign.due_date.date_naive() - now).num_seconds();

			// only write if the deadline hasn't passed, or if it's not more than a week away from today
			if offset >= -(60 * 60 * 24 * 3) && offset < 60 * 60 * 24 * 7 {
				Some(format!("{:<32} {}\n",
						assign.name,
						assign.due_date.format("Due %B %e, %l:%M %p"))
					.to_string())
			}
			else {
				None
			}
		})
		.fold(String::new(), |prev, s| prev + &s)
}

fn select_class(s: &mut Cursive, name: Arc<String>) {
	let list = {
		let todolist = s.user_data::<RefCell<TodoList>>().unwrap().borrow();
		get_assign_text(&todolist, (*name).clone())
	};
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
		let res = {
			let mut todolist = s.user_data::<RefCell<TodoList>>().unwrap().borrow_mut();
			todolist.new_class(name.to_string())
		};

		match res {
			Ok(_) => {
				s.call_on_name("select", |view: &mut SelectView<String>| {
					view.add_item_str(name);
				});
			},
			Err(_) => (),
		}
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

			let todolist_ref = s.user_data::<RefCell<TodoList>>().unwrap().clone();
			let assign_text = {
				let mut todolist = todolist_ref.borrow_mut();

				let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
				let time = NaiveTime::parse_from_str(&time_str, "%H:%M").ok();
				let uid_opt = if let (Some(good_date), Some(good_time)) = (date, time) {
					let due_date = NaiveDateTime::new(good_date, good_time)
						.and_local_timezone(Local)
						.unwrap();
					Some(todolist.new_assignment(classname.to_string(), Assignment {
							due_date,
							name: (*name).clone(),
							completed: false,
						}).unwrap())
				}
				else {
					let dialog = Dialog::around(TextView::new("Formating error with date/time")).button("Ok", Cursive::noop);
					s.add_layer(dialog);
					s.pop_layer();
					None
				};

				match uid_opt {
					Some(_) => (),
					None => {
						let dialog = Dialog::around(TextView::new("Failed to add new assignment successfully")).button("Ok", Cursive::noop);
						s.add_layer(dialog);
						s.pop_layer();
					}
				}

				s.pop_layer();
				get_assign_text(&todolist, (*classname).clone())
			};

			s.call_on_name("weektodo", move |list: &mut LinearLayout| {
				let todolist = todolist_ref.borrow();
				make_todo_list(&todolist, list);
			});
			s.call_on_name("assigns", move |list: &mut TextView| {
				list.set_content(assign_text);
			});
		})
		.button("Cancel", |s| {
			s.pop_layer();
		}));
}
