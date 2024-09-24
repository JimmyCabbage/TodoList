use std::fs;
use std::env;
use std::vec::Vec;
use std::collections::HashMap;
use std::io::{prelude::*, BufReader};
use std::clone::Clone;
use std::cmp::{PartialEq,Eq};
use chrono::{prelude::*, DateTime, NaiveDateTime, NaiveDate, NaiveTime};

mod assignment;
mod todolist;

use assignment::Assignment;
use todolist::TodoList;

fn main() {
	let listpath = env::var("HOME").unwrap() + "/.todolist";
	let mut todolist = TodoList::new(listpath).unwrap();

	let now = Local::now();
	for (class, assignments) in &todolist.assignments_by_class {
		println!("{}", class);
		for assign in assignments {
			let offset = (assign.due_date - now).num_seconds();
			if offset >= 0 && offset < 60 * 60 * 24 * 7 {
				println!("{:<20} {}", assign.name, assign.due_date.format("Due %B %e, %l:%M %p"));
			}
		}
	}

	/*let now = Local::now();
	let mut prev_class = String::from("");
	for assign in &assignments {

		// only write if the deadline hasn't passed, or if it's not more than a week away from today
		if offset >= 0 && offset < 60 * 60 * 24 * 7 {
			//println!("{} {} {}", assign.due_date.format("Due %B %e, %l:%M %p"), assign.name, assign.classname);
			if prev_class != assign.classname {
				ui.heading(&assign.classname);

				prev_class = assign.classname.clone();
			}

			ui.label(format!("{:<20} {}\n", assign.name, assign.due_date.format("due %b %e, %l:%M %p")));
		}
	}*/
}
