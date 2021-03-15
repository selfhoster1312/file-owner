use nix::unistd::chown;
use nix::unistd::{Gid, Uid, Group as NixGroup, User};
use libc;
use std::path::Path;
use std::fmt::{self, Display};
use std::error::Error;
use std::convert::{TryFrom, TryInto, Infallible};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
pub enum FileOwnerError {
    IoError(io::Error),
    NixError(nix::Error),
    UserNotFound(String),
    GroupNotFound(String),
}

impl Display for FileOwnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileOwnerError::IoError(_) => write!(f, "I/O error"),
            FileOwnerError::NixError(_) => write!(f, "*nix error"),
            FileOwnerError::UserNotFound(name) => write!(f, "user name {:?} not found", name),
			FileOwnerError::GroupNotFound(name) => write!(f, "group name {:?} not found", name),
        }
    }
}

impl Error for FileOwnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileOwnerError::IoError(err) => Some(err),
            FileOwnerError::NixError(err) => Some(err),
            FileOwnerError::UserNotFound(_) => None,
			FileOwnerError::GroupNotFound(_) => None,
        }
    }
}

impl From<io::Error> for FileOwnerError {
    fn from(err: io::Error) -> FileOwnerError {
        FileOwnerError::IoError(err)
    }
}

impl From<nix::Error> for FileOwnerError {
    fn from(err: nix::Error) -> FileOwnerError {
        FileOwnerError::NixError(err)
    }
}

impl From<Infallible> for FileOwnerError {
    fn from(_err: Infallible) -> FileOwnerError {
        unreachable!()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Owner(Uid);

impl Owner {
    pub fn from_uid(uid: libc::uid_t) -> Owner {
        Owner(Uid::from_raw(uid))
    }

    pub fn from_name(user: &str) -> Result<Owner, FileOwnerError> {
        Ok(Owner(User::from_name(user)?.ok_or_else(|| FileOwnerError::UserNotFound(user.to_owned()))?.uid))
    }

    pub fn name(&self) -> Result<Option<String>, FileOwnerError> {
        Ok(User::from_uid(self.0)?.map(|u| u.name))
    }
}

impl From<libc::uid_t> for Owner {
    fn from(uid: libc::uid_t) -> Owner {
        Owner::from_uid(uid)
    }
}

impl<'s> TryFrom<&'s str> for Owner {
    type Error = FileOwnerError;

    fn try_from(name: &'s str) -> Result<Owner, Self::Error> {
        Owner::from_name(name)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Group(Gid);

impl Group {
    pub fn from_gid(gid: libc::gid_t) -> Group {
        Group(Gid::from_raw(gid))
    }

    pub fn from_name(group: &str) -> Result<Group, FileOwnerError> {
        Ok(Group(NixGroup::from_name(group)?.ok_or_else(|| FileOwnerError::GroupNotFound(group.to_owned()))?.gid))
    }

    pub fn name(&self) -> Result<Option<String>, FileOwnerError> {
        Ok(NixGroup::from_gid(self.0)?.map(|u| u.name))
    }
}

impl From<libc::gid_t> for Group {
    fn from(gid: libc::gid_t) -> Group {
        Group::from_gid(gid)
    }
}

impl<'s> TryFrom<&'s str> for Group {
    type Error = FileOwnerError;

    fn try_from(name: &'s str) -> Result<Group, Self::Error> {
        Group::from_name(name)
    }
}

pub fn set_owner<E: Into<FileOwnerError>>(path: impl AsRef<Path>, owner: impl TryInto<Owner, Error = E>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref().into(), Some(owner.try_into().map_err(Into::into)?.0), None)?)
}

pub fn set_group<E: Into<FileOwnerError>>(path: impl AsRef<Path>, group: impl TryInto<Group, Error = E>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref().into(), None, Some(group.try_into().map_err(Into::into)?.0))?)
}

pub fn set_owner_group<E1: Into<FileOwnerError>, E2: Into<FileOwnerError>>(path: impl AsRef<Path>, owner: impl TryInto<Owner, Error = E1>, group: impl TryInto<Group, Error = E2>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref().into(), Some(owner.try_into().map_err(Into::into)?.0), Some(group.try_into().map_err(Into::into)?.0))?)
}

pub fn owner(path: impl AsRef<Path>) -> Result<Owner, FileOwnerError> {
    Ok(Owner::from_uid(fs::metadata(path)?.uid() as libc::uid_t))
}

pub fn group(path: impl AsRef<Path>) -> Result<Group, FileOwnerError> {
    Ok(Group::from_gid(fs::metadata(path)?.gid() as libc::gid_t))
}

pub fn owner_group(path: impl AsRef<Path>) -> Result<(Owner, Group), FileOwnerError> {
    let meta = fs::metadata(path)?;
    Ok((Owner::from_uid(meta.uid() as libc::uid_t), Group::from_gid(meta.gid() as libc::gid_t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set() {
        std::fs::write("/tmp/foo", "test").unwrap();

        set_owner("/tmp/foo", "nobody").unwrap();
        set_owner("/tmp/foo", 99).unwrap();

        set_group("/tmp/foo", "nogroup").unwrap();
        set_group("/tmp/foo", 99).unwrap();

        set_owner_group("/tmp/foo", "nobody", "nogroup").unwrap();
        set_owner_group("/tmp/foo", 99, 99).unwrap();
        set_owner_group("/tmp/foo", 99, "nogroup").unwrap();
        set_owner_group("/tmp/foo", "nobody", 99).unwrap();
    }

    #[test]
    fn test_get() {
        std::fs::write("/tmp/bar", "test").unwrap();

        set_owner("/tmp/bar", "nobody").unwrap();
        set_group("/tmp/bar", "nogroup").unwrap();

        assert_eq!(owner("/tmp/bar").unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(group("/tmp/bar").unwrap().name().unwrap().as_deref(), Some("nogroup"));

        let (o, g) = owner_group("/tmp/bar").unwrap();
        assert_eq!(o.name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(g.name().unwrap().as_deref(), Some("nogroup"));
    }
}
