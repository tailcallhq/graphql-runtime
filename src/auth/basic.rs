use headers::authorization::Basic;
use headers::{Authorization, HeaderMapExt};
use htpasswd_verify::Htpasswd;

use super::error::Error;
use super::verify::Verify;
use crate::blueprint;
use crate::http::RequestContext;

pub struct BasicVerifier {
  verifier: Htpasswd<'static>,
}

impl Verify for BasicVerifier {
  /// Verify the request context against the basic auth provider.
  async fn verify(&self, req_ctx: &RequestContext) -> Result<(), Error> {
    let header = req_ctx.req_headers.typed_get::<Authorization<Basic>>();

    let Some(header) = header else {
      return Err(Error::Missing);
    };

    if self.verifier.check(header.username(), header.password()) {
      Ok(())
    } else {
      Err(Error::Invalid)
    }
  }
}

impl BasicVerifier {
  pub fn new(options: blueprint::BasicProvider) -> Self {
    Self { verifier: Htpasswd::new_owned(&options.htpasswd) }
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;

  // testuser1:password123
  // testuser2:mypassword
  // testuser3:abc123
  pub static HTPASSWD_TEST: &str = "
testuser1:$apr1$e3dp9qh2$fFIfHU9bilvVZBl8TxKzL/
testuser2:$2y$10$wJ/mZDURcAOBIrswCAKFsO0Nk7BpHmWl/XuhF7lNm3gBAFH3ofsuu
testuser3:{SHA}Y2fEjdGT1W6nsLqtJbGUVeUp9e4=
";

  pub fn create_basic_auth_request(username: &str, password: &str) -> RequestContext {
    let mut req_context = RequestContext::default();

    req_context
      .req_headers
      .typed_insert(Authorization::basic(username, password));

    req_context
  }

  #[tokio::test]
  async fn verify_passwords() {
    let provider = BasicVerifier::new(blueprint::BasicProvider { htpasswd: HTPASSWD_TEST.to_owned() });

    let validation = provider.verify(&RequestContext::default()).await.err();
    assert_eq!(validation, Some(Error::Missing));

    let validation = provider
      .verify(&create_basic_auth_request("testuser1", "wrong-password"))
      .await
      .err();
    assert_eq!(validation, Some(Error::Invalid));

    let validation = provider
      .verify(&create_basic_auth_request("testuser1", "password123"))
      .await;
    assert!(validation.is_ok());

    let validation = provider
      .verify(&create_basic_auth_request("testuser2", "mypassword"))
      .await;
    assert!(validation.is_ok());

    let validation = provider.verify(&create_basic_auth_request("testuser3", "abc123")).await;
    assert!(validation.is_ok());
  }
}
