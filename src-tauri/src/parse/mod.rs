//! Parser für Moodle-Seiten. Reine Funktionen, ohne Netzwerk.

mod course;
mod files;
mod session_page;
#[cfg(test)]
mod tests;

pub use course::{merge_extra_files, parse_course_page, parse_course_state, RawItem};
pub use files::{find_embedded_file, parse_assign_page, parse_folder_page, parse_url_page};
pub use session_page::{is_logged_in, parse_courses_html, parse_sesskey, parse_user};
