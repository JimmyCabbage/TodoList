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

use chrono::{prelude::*, DateTime};
use std::hash::Hash;
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Hash, Serialize, Deserialize)]
pub struct Assignment {
	pub due_date: DateTime<Local>,
	pub name: String,
}

impl Clone for Assignment {
	fn clone(&self) -> Self {
		Self {
			due_date: self.due_date.clone(),
			name: self.name.clone(),
		}
	}
}

impl PartialEq for Assignment {
	fn eq(&self, other: &Self) -> bool {
		self.due_date == other.due_date &&
			self.name == other.name
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

#[derive(Hash, Serialize, Deserialize)]
pub struct AssignmentV1 {
	pub due_date: DateTime<Local>,
	pub name: String,
	pub completed: bool,
}

impl Clone for AssignmentV1 {
	fn clone(&self) -> Self {
		Self {
			due_date: self.due_date.clone(),
			name: self.name.clone(),
			completed: self.completed,
		}
	}
}

impl PartialEq for AssignmentV1 {
	fn eq(&self, other: &Self) -> bool {
		self.due_date == other.due_date &&
			self.name == other.name &&
			self.completed == other.completed
	}
}

impl Eq for AssignmentV1 {
}

impl Ord for AssignmentV1 {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.due_date == other.due_date {
			self.name.cmp(&other.name)
		}
		else {
			self.due_date.cmp(&other.due_date)
		}
	}
}

impl PartialOrd for AssignmentV1 {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
