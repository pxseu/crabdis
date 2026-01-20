use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

pub struct Auth {
    password: Option<Arc<str>>,
}
pub type AuthRef = Arc<Auth>;

impl Auth {
    pub fn new(password: Option<&str>) -> AuthRef {
        log::info!(
            "Authentication status: {}",
            if password.is_some() {
                "enabled"
            } else {
                "disabled"
            }
        );

        Arc::new(Self {
            password: password.map(Arc::from),
        })
    }

    #[must_use]
    pub const fn has_auth(&self) -> bool {
        self.password.is_some()
    }

    pub fn login(&self, username: &str, password: &str) -> Result<()> {
        if username != "default" {
            return Err(IoError::new(ErrorKind::PermissionDenied, "Invalid username").into());
        }

        if let Some(expected_password) = self.password.as_deref()
            && !crabdis_core::eq::constant_time_str(expected_password, password)
        {
            return Err(IoError::new(ErrorKind::PermissionDenied, "Invalid password").into());
        }

        Ok(())
    }
}
