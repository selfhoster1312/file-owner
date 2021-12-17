/*!
Set and get Unix file owner and group.

UID/GUI numbers or user/group names can be used.

Note: This crate will only compile on Unix systems.

# Usage examples

## Set owner and group by name

```no_run
use file_owner::PathExt;

"/tmp/baz".set_owner("nobody").unwrap();
"/tmp/baz".set_group("nogroup").unwrap();
```

## Set owner and group by id

```no_run
use file_owner::PathExt;

"/tmp/baz".set_owner(99).unwrap();
"/tmp/baz".set_group(99).unwrap();
```

## Get owner and group

```no_run
use file_owner::PathExt;

let o = "/tmp/baz".owner().unwrap();
o.id(); // 99
o.name(); // Some("nobody")

let g = "/tmp/baz".group().unwrap();
g.id(); // 99
g.name(); // Some("nogroup")
```
*/

use nix::unistd::chown;
use nix::unistd::{Gid, Uid, Group as NixGroup, User};
use std::path::Path;
use std::fmt::{self, Display};
use std::error::Error;
use std::convert::{TryFrom, TryInto, Infallible};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;

/// File owner or group error.
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

/// Owner of a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Owner(Uid);

impl Owner {
    /// Constructs Owner from UID.
    pub fn from_uid(uid: u32) -> Owner {
        Owner(Uid::from_raw(uid.try_into().unwrap()))
    }

    /// Construct Owner from name.
    pub fn from_name(user: &str) -> Result<Owner, FileOwnerError> {
        Ok(Owner(User::from_name(user)?.ok_or_else(|| FileOwnerError::UserNotFound(user.to_owned()))?.uid))
    }

    /// Gets UID.
    pub fn id(&self) -> u32 {
        self.0.as_raw().try_into().unwrap()
    }

    /// Gets name if assigned to UID.
    pub fn name(&self) -> Result<Option<String>, FileOwnerError> {
        Ok(User::from_uid(self.0)?.map(|u| u.name))
    }
}

impl From<u32> for Owner {
    fn from(uid: u32) -> Owner {
        Owner::from_uid(uid)
    }
}

impl<'s> TryFrom<&'s str> for Owner {
    type Error = FileOwnerError;

    fn try_from(name: &'s str) -> Result<Owner, Self::Error> {
        Owner::from_name(name)
    }
}

impl Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name().ok().flatten() {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", self.id())
        }
    }
}

/// Group of a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Group(Gid);

impl Group {
    /// Constructs Group from GUI.
    pub fn from_gid(gid: u32) -> Group {
        Group(Gid::from_raw(gid.try_into().unwrap()))
    }

    /// Constructs Group from name.
    pub fn from_name(group: &str) -> Result<Group, FileOwnerError> {
        Ok(Group(NixGroup::from_name(group)?.ok_or_else(|| FileOwnerError::GroupNotFound(group.to_owned()))?.gid))
    }

    /// Gets GID.
    pub fn id(&self) -> u32 {
        self.0.as_raw().try_into().unwrap()
    }

    /// Gets name if assigned to GID.
    pub fn name(&self) -> Result<Option<String>, FileOwnerError> {
        Ok(NixGroup::from_gid(self.0)?.map(|u| u.name))
    }
}

impl From<u32> for Group {
    fn from(gid: u32) -> Group {
        Group::from_gid(gid)
    }
}

impl<'s> TryFrom<&'s str> for Group {
    type Error = FileOwnerError;

    fn try_from(name: &'s str) -> Result<Group, Self::Error> {
        Group::from_name(name)
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name().ok().flatten() {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", self.id())
        }
    }
}

/// Sets owner to file at the given path.
pub fn set_owner<E: Into<FileOwnerError>>(path: impl AsRef<Path>, owner: impl TryInto<Owner, Error = E>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref(), Some(owner.try_into().map_err(Into::into)?.0), None)?)
}

/// Sets group to file at the given path.
pub fn set_group<E: Into<FileOwnerError>>(path: impl AsRef<Path>, group: impl TryInto<Group, Error = E>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref(), None, Some(group.try_into().map_err(Into::into)?.0))?)
}

/// Sets owner and group to file at the given path.
pub fn set_owner_group<E1: Into<FileOwnerError>, E2: Into<FileOwnerError>>(path: impl AsRef<Path>, owner: impl TryInto<Owner, Error = E1>, group: impl TryInto<Group, Error = E2>) -> Result<(), FileOwnerError> {
    Ok(chown(path.as_ref(), Some(owner.try_into().map_err(Into::into)?.0), Some(group.try_into().map_err(Into::into)?.0))?)
}

/// Gets owner of a file at the given path.
pub fn owner(path: impl AsRef<Path>) -> Result<Owner, FileOwnerError> {
    Ok(Owner::from_uid(fs::metadata(path)?.uid().try_into().unwrap()))
}

/// Gets group of a file at the given path.
pub fn group(path: impl AsRef<Path>) -> Result<Group, FileOwnerError> {
    Ok(Group::from_gid(fs::metadata(path)?.gid().try_into().unwrap()))
}

/// Gets owner and group of a file at the given path.
pub fn owner_group(path: impl AsRef<Path>) -> Result<(Owner, Group), FileOwnerError> {
    let meta = fs::metadata(path)?;
    Ok((Owner::from_uid(meta.uid().try_into().unwrap()), Group::from_gid(meta.gid().try_into().unwrap())))
}

/// Extension methods for `T: AsRef<Path>`.
pub trait PathExt {
    /// Sets owner to file at the given path.
    fn set_owner<E: Into<FileOwnerError>>(&self, owner: impl TryInto<Owner, Error = E>) -> Result<(), FileOwnerError>;

    /// Sets group to file at the given path.
    fn set_group<E: Into<FileOwnerError>>(&self, group: impl TryInto<Group, Error = E>) -> Result<(), FileOwnerError>;

    /// Sets owner and group to file at the given path.
    fn set_owner_group<E1: Into<FileOwnerError>, E2: Into<FileOwnerError>>(&self, owner: impl TryInto<Owner, Error = E1>, group: impl TryInto<Group, Error = E2>) -> Result<(), FileOwnerError>;

    /// Gets owner of a file at the given path.
    fn owner(&self) -> Result<Owner, FileOwnerError>;

    /// Gets group of a file at the given path.
    fn group(&self) -> Result<Group, FileOwnerError>;

    /// Gets owner and group of a file at the given path.
    fn owner_group(&self) -> Result<(Owner, Group), FileOwnerError>;
}

impl<T: AsRef<Path>> PathExt for T {
    fn set_owner<E: Into<FileOwnerError>>(&self, owner: impl TryInto<Owner, Error = E>) -> Result<(), FileOwnerError> {
        set_owner(self, owner)
    }

    fn set_group<E: Into<FileOwnerError>>(&self, group: impl TryInto<Group, Error = E>) -> Result<(), FileOwnerError> {
        set_group(self, group)
    }

    fn set_owner_group<E1: Into<FileOwnerError>, E2: Into<FileOwnerError>>(&self, owner: impl TryInto<Owner, Error = E1>, group: impl TryInto<Group, Error = E2>) -> Result<(), FileOwnerError> {
        set_owner_group(self, owner, group)
    }

    fn owner(&self) -> Result<Owner, FileOwnerError> {
        owner(self)
    }

    fn group(&self) -> Result<Group, FileOwnerError> {
        group(self)
    }

    fn owner_group(&self) -> Result<(Owner, Group), FileOwnerError> {
        owner_group(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let nobody_id = Owner::from_name("nobody").unwrap().id();
        let nogroup_id = Group::from_name("nogroup").unwrap().id();

        assert_eq!(&Owner::from_uid(nobody_id).to_string(), "nobody");
        assert_eq!(&Group::from_gid(nogroup_id).to_string(), "nogroup");

        assert_eq!(&Owner::from_uid(321321).to_string(), "321321");
        assert_eq!(&Group::from_gid(321321).to_string(), "321321");
    }

    #[test]
    #[ignore]
    fn test_set_get() {
        let nobody_id = Owner::from_name("nobody").unwrap().id();
        let nogroup_id = Group::from_name("nogroup").unwrap().id();

        let file1 = tempfile::NamedTempFile::new().unwrap();
        let file_path1 = file1.path();
        let file2 = tempfile::NamedTempFile::new().unwrap();
        let file_path2 = file2.path();

        set_owner(file_path1, "nobody").unwrap();
        assert_eq!(owner(file_path1).unwrap().name().unwrap().as_deref(), Some("nobody"));
        set_owner(file_path2, nobody_id).unwrap();
        assert_eq!(owner(file_path2).unwrap().name().unwrap().as_deref(), Some("nobody"));

        set_group(file_path1, "nogroup").unwrap();
        assert_eq!(group(file_path1).unwrap().name().unwrap().as_deref(), Some("nogroup"));
        set_group(file_path2, nogroup_id).unwrap();
        assert_eq!(group(file_path2).unwrap().name().unwrap().as_deref(), Some("nogroup"));
    }

    #[test]
    #[ignore]
    fn test_set_get_all() {
        let nobody_id = Owner::from_name("nobody").unwrap().id();
        let nogroup_id = Group::from_name("nogroup").unwrap().id();

        let file1 = tempfile::NamedTempFile::new().unwrap();
        let file_path1 = file1.path();
        let file2 = tempfile::NamedTempFile::new().unwrap();
        let file_path2 = file2.path();
        let file3 = tempfile::NamedTempFile::new().unwrap();
        let file_path3 = file3.path();
        let file4 = tempfile::NamedTempFile::new().unwrap();
        let file_path4 = file4.path();

        set_owner_group(file_path1, "nobody", "nogroup").unwrap();
        assert_eq!(owner(file_path1).unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(group(file_path1).unwrap().name().unwrap().as_deref(), Some("nogroup"));
        set_owner_group(file_path2, nobody_id, nogroup_id).unwrap();
        assert_eq!(owner(file_path2).unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(group(file_path2).unwrap().name().unwrap().as_deref(), Some("nogroup"));

        set_owner_group(file_path3, nobody_id, "nogroup").unwrap();
        assert_eq!(owner(file_path3).unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(group(file_path3).unwrap().name().unwrap().as_deref(), Some("nogroup"));
        set_owner_group(file_path4, "nobody", nogroup_id).unwrap();
        assert_eq!(owner(file_path4).unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(group(file_path4).unwrap().name().unwrap().as_deref(), Some("nogroup"));
    }

    #[test]
    #[ignore]
    fn test_get_all() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let file_path = file.path();

        set_owner_group(file_path, "nobody", "nogroup").unwrap();

        let (o, g) = owner_group(file_path).unwrap();
        assert_eq!(o.name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(g.name().unwrap().as_deref(), Some("nogroup"));
    }

    #[test]
    #[ignore]
    fn test_ext_traits() {
        let nobody_id = Owner::from_name("nobody").unwrap().id();
        let nogroup_id = Group::from_name("nogroup").unwrap().id();

        let file = tempfile::NamedTempFile::new().unwrap();
        let file_path = file.path();

        file_path.set_owner("nobody").unwrap();
        file_path.set_group("nogroup").unwrap();

        assert_eq!(file_path.owner().unwrap().name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(file_path.group().unwrap().name().unwrap().as_deref(), Some("nogroup"));

        file_path.set_owner_group("nobody", "nogroup").unwrap();

        let (o, g) = file_path.owner_group().unwrap();
        assert_eq!(o.name().unwrap().as_deref(), Some("nobody"));
        assert_eq!(g.name().unwrap().as_deref(), Some("nogroup"));

        assert_eq!(o.id(), nobody_id);
        assert_eq!(g.id(), nogroup_id);
    }
}
