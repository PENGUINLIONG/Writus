use writium_auth::*;
use writium::prelude::*;
use hyper::header::{Authorization, Bearer};

pub struct SimpleAuthority {
    cred: Option<String>,
}
impl SimpleAuthority {
    pub fn new(cred: &str) -> SimpleAuthority {
        SimpleAuthority {
            cred: Some(cred.to_string()),
        }
    }
}
fn gen_unauthorized(msg: &'static str) -> Error {
    Error::unauthorized(msg)
        .with_header(
            WwwAuthenticate::new()
                .with_challenge(
                    Challenge::new("Bearer")
                        .with_param("realm", "Unsafe HTTP method.")
                )
        )
}
impl Authority for SimpleAuthority {
    type Privilege = ();
    fn authorize(&self, _pr: (), req: &Request) -> Result<()> {
        if self.cred.is_none() {
            return Err(gen_unauthorized("No credential to be matched. Maybe the administrator intended to do so. For safety reasons, any authentication request is rejected."))
        }
        if let Some(cr) = req.header::<Authorization<Bearer>>() {
            if self.cred.as_ref().unwrap() == &*cr.0.token {
               Ok(())
            } else {
               Err(gen_unauthorized("Unauthorized access. Please check if your identity token is correct."))
            }
        } else {
           Err(gen_unauthorized("This request needs certain priviledge to be proceeded. Please provide your identity token in header `Authorization`."))
        }
    }
}
impl Default for SimpleAuthority {
    fn default() -> SimpleAuthority {
        SimpleAuthority {
            cred: None,
        }
    }
}
