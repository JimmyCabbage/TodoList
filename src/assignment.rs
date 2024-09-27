use chrono::{prelude::*, DateTime};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::cmp::Ordering;

#[derive(Hash)]
pub struct Assignment {
	pub due_date: DateTime<Local>,
	pub name: String,
	pub completed: bool,
}
	/*fn load_from_file<P>(path: P) -> Vec<Assignment>
		where P: AsRef<Path>
	{
		// we load the file before waiting on a modification
		let mut assignments = Vec::new();

		// reload all assignments from file
		let list_file = File::open(path).unwrap();
		let list_reader = BufReader::new(list_file);
		for line in list_reader.lines() {
			let line = line.unwrap();

			let values = line.split(",").collect::<Vec<_>>();
			if values.len() >= 4 {
				let date = NaiveDate::parse_from_str(values[0], "%Y-%m-%d").unwrap();
				let time = NaiveTime::parse_from_str(values[1], "%H:%M").unwrap();
				assignments.push(Assignment {
					due_date: NaiveDateTime::new(date, time)
						.and_local_timezone(Local)
						.unwrap(),
					classname: String::from(values[2]),
					name: String::from(values[3]),
				});
			}
		}

		// sort by classname & due date
		assignments.sort_by(|a,b| {
			let a_due = a.due_date.timestamp();
			let b_due = b.due_date.timestamp();
			if a.classname == b.classname {
				return a_due.cmp(&b_due);
			}

			return a.classname.cmp(&b.classname);
		});

		return assignments;
	}*/

impl Clone for Assignment {
	fn clone(&self) -> Self {
		Self {
			due_date: self.due_date.clone(),
			name: self.name.clone(),
			completed: self.completed,
		}
	}
}

impl PartialEq for Assignment {
	fn eq(&self, other: &Self) -> bool {
		self.due_date == other.due_date &&
			self.name == other.name &&
			self.completed == other.completed
	}
}

impl Eq for Assignment {
}

impl Ord for Assignment {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.due_date == other.due_date {
			self.name.cmp(&other.name)
		}
		else {
			self.due_date.cmp(&other.due_date)
		}
	}
}

impl PartialOrd for Assignment {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
