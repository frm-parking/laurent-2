#[macro_export]
macro_rules! as_match {
  ($expr:expr) => {
    $expr.iter().map(AsRef::<str>::as_ref).collect::<Vec<_>>()[..]
  };
}

pub fn is_event(part: &str) -> bool {
	part == "#M"
}
