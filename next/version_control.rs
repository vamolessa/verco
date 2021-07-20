pub mod git;

pub trait VersionControl {}

pub fn version_control_from_current_repository() -> Box<dyn VersionControl> {
    Box::new(git::Git::new())
}

