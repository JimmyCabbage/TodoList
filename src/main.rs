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

use std::env;
use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;
use std::vec::Vec;
use std::hash::{DefaultHasher, Hash, Hasher};
use chrono::{prelude::*, NaiveDateTime, NaiveDate, NaiveTime, Days};
use cursive::Cursive;
use cursive::views::{Button, Dialog, DummyView, EditView, TextView, LinearLayout, SelectView, ScrollView, Checkbox};
use cursive::traits::*;

mod assignment;
mod todolist;

use assignment::Assignment;
use todolist::TodoList;

fn main() {
	let listpath = env::var("HOME").unwrap() + "/.todolist";
	let scriptpath = env::var("HOME").unwrap() + "/.todolistrc";
	let todolist = Arc::new(RefCell::new(TodoList::new(listpath, scriptpath).unwrap()));

	let mut siv = cursive::default();
	siv.set_user_data(todolist.clone());

	let classes_view = {
		let mut classes_view = SelectView::<String>::new();
		make_class_view(&todolist.borrow(), &mut classes_view);

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
		.child(Button::new("Save", |s| {
			{
				let todolist = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().borrow();
				todolist.save_to_file();
			}
			s.add_layer(Dialog::around(TextView::new("Saved list to file successfully!"))
				.button("OK", |s| {
					s.pop_layer();
				}));
		}))
		.child(DummyView)
		.child(Button::new("Quit", Cursive::quit));

	siv.add_layer(Dialog::around(LinearLayout::vertical()
		.child(info_view)
		.child(DummyView)
		.child(buttons)));

	//let main_menu = Menubar::new()
		//.insert(

	siv.run();

	eprintln!("Successfully exited.");
}

fn make_class_view(todolist: &TodoList, classes_view: &mut SelectView<String>) {
	classes_view.clear();
	classes_view.add_all_str(todolist.get_classes());
}

fn make_todo_list(todolist: &TodoList, vert: &mut LinearLayout) {
	let now = Local::now().date_naive();
	let assignments_by_date = {
		let all_week_assignments = {
			let begin = now.checked_sub_days(Days::new(3)).unwrap();
			let end = now.checked_add_days(Days::new(24)).unwrap();
			todolist.get_timespan_assignments(begin, end)
		};

		let mut date_assign = HashMap::new();
		for i in -3i64..=10 {
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
			// sort first by due date, then by classname, then by assign. name
			assignments.sort_by(|(ca, a), (cb, b)| {
				let a_due = a.due_date;
				let b_due = b.due_date;
				if a_due != b_due {
					a.due_date.cmp(&b.due_date)
				}
				else if ca != cb {
					ca.cmp(&cb)
				}
				else {
					a.name.cmp(&b.name)
				}
			});
			assignments
		};

		// add small little notices of the especially important dates
		let notice = {
			if *date == now {
				" (TODAY)"
			}
			else if date.checked_sub_days(Days::new(1)).unwrap() == now {
				" (TOMORROW)"
			}
			else {
				""
			}
		};
		vert.add_child(TextView::new(format!("{}{}", date.format("Due %a, %b %e").to_string(), &notice)));

		let time_format_str = "%l:%M %p";

		let classname_len = 8;
		let max_assign_name_len = 32;
		let banner = "─".repeat(4) + "┬" + &"─".repeat(time_format_str.len() + 2) + "┬" + &"─".repeat(classname_len + 2) + "┬" + &"─".repeat(max_assign_name_len + 1);
		vert.add_child(TextView::new(banner).no_wrap());
		for (classname, assign) in assignments {
			let due_date = assign.due_date.format(time_format_str).to_string();
			let uid = {
				let mut h = DefaultHasher::new();
				assign.hash(&mut h);
				h.finish()
			};

			// this should probably error out, but it does
			if let Ok(already_completed) = todolist.get_assignment_completion(uid) {
				let check = Checkbox::new().with_checked(already_completed)
					.on_change(move |s, checked| {
						let mut todolist = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().borrow_mut();
						todolist.set_assignment_completion(uid, checked).unwrap();
					});

				vert.add_child(LinearLayout::horizontal()
					.child(check)
					.child(TextView::new(" │ ").no_wrap())
					.child(TextView::new(due_date).no_wrap())
					.child(TextView::new(" │ ").no_wrap())
					.child(ScrollView::new(TextView::new(classname).no_wrap().min_width(classname_len).max_width(classname_len)))
					.child(TextView::new(" │ ").no_wrap())
					.child(ScrollView::new(TextView::new(assign.name).no_wrap().max_width(max_assign_name_len))));
			}
		}
		vert.add_child(DummyView);
	}
}

// returns a string of all assigments from 3 days ago to infinity
// each assignment is seperated by newline
fn get_assign_text(todolist: &TodoList, classname: String) -> String {
	let assignments = {
		let mut assignments = todolist.get_class_assignments(&classname).unwrap();
		assignments.sort();
		assignments
	};
	let now = Local::now().date_naive();

	// transform each assignment into a string, then move each string into a megastring to return
	assignments.iter()
		.filter_map(|assign| {
			let offset = (assign.due_date.date_naive() - now).num_seconds();

			// only write if it's not more than 3 days earlier than today
			if offset >= -(60 * 60 * 24 * 3) {
				Some(format!("{:<32} {}\n",
						assign.name,
						assign.due_date.format("Due %a, %B %e, %l:%M %p"))
					.to_string())
			}
			else {
				None
			}
		})
		.fold(String::new(), |prev, s| prev + &s)
}

// opens up a menu with information and modifiers on this specific class targeted by name
fn select_class(s: &mut Cursive, name: Arc<String>) {
	let list = {
		let todolist = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().borrow();
		get_assign_text(&todolist, (*name).clone())
	};
	let text_view = TextView::new(list)
		.with_name("assigns");

	let add = {
		let name = name.clone();
		move |s: &mut Cursive| {
			add_assignment(s, name.clone());
		}
	};
	let rm = {
		let name = name.clone();
		move |s: &mut Cursive| {
			let name = name.clone();
			s.add_layer(Dialog::around(TextView::new(format!("Are you sure you want to delete class \"{}\"", name.clone())))
				.button("Cancel", |s| {
					s.pop_layer();
				})
				.button("Delete", move |s| {
					{
						let todolist_ref = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().clone();
						{
							let mut todolist = todolist_ref.borrow_mut();
							todolist.delete_class((*name).clone()).unwrap();
						}

						s.call_on_name("select", |view: &mut SelectView<String>| {
							make_class_view(&todolist_ref.borrow(), view);
						});
						s.pop_layer();
					}
					s.pop_layer();
				}));
		}
	};
	s.add_layer(Dialog::around(ScrollView::new(text_view))
		.button("Add new assignment", add)
		.button("Delete this class", rm)
		.button("OK", |s| {
			s.pop_layer();
		}));
}

fn add_classname(s: &mut Cursive) {
	fn ok(s: &mut Cursive, name: &str) {
		let res = {
			let mut todolist = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().borrow_mut();
			todolist.create_class(name.to_string())
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
	let tomorrow = Local::now().date_naive().checked_add_days(Days::new(1)).unwrap();
	let name = EditView::new()
		.with_name("new_name")
		.fixed_width(20);
	let date = EditView::new()
		.content(tomorrow.format("%Y-%m-%d").to_string())
		.with_name("date")
		.fixed_width(11);
	let time = EditView::new()
		.content("08:00")
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

			let todolist_ref = s.user_data::<Arc<RefCell<TodoList>>>().unwrap().clone();
			let assign_text = {
				let mut todolist = todolist_ref.borrow_mut();

				let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
				let time = NaiveTime::parse_from_str(&time_str, "%H:%M").ok();
				let uid_opt = if let (Some(good_date), Some(good_time)) = (date, time) {
					let due_date = NaiveDateTime::new(good_date, good_time)
						.and_local_timezone(Local)
						.unwrap();
					Some(todolist.create_assignment(classname.to_string(), Assignment {
							due_date,
							name: (*name).clone(),
						}).unwrap())
				}
				else {
					let dialog = Dialog::around(TextView::new("Formating error with date/time")).button("Ok", |s| {
						s.pop_layer();
					});
					s.add_layer(dialog);
					None
				};

				match uid_opt {
					Some(_) => {
						s.pop_layer();
					}
					None => {
						let dialog = Dialog::around(TextView::new("Failed to add new assignment")).button("Ok", |s| {
							s.pop_layer();
						});
						s.add_layer(dialog);
						s.pop_layer();
					}
				}

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
