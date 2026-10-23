// SPDX-License-Identifier: LGPL-3.0-or-later
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]

pub mod convert;
pub mod types;
pub mod winuser;

#[cfg(not(test))]
pub mod entry;
