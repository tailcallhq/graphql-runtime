use std::collections::HashSet;
use std::time::Duration;

use jsonwebtoken::jwk::JwkSet;
use url::Url;

use super::init_context::InitContext;
use crate::config;
use crate::directive::DirectiveCodec;
use crate::mustache::Mustache;
use crate::valid::{Valid, ValidationError};

#[derive(Debug, Clone)]
pub struct BasicProvider {
  pub htpasswd: String,
}

#[derive(Debug, Clone)]
pub enum Jwks {
  Local(JwkSet),
  Remote { url: Url, max_age: Duration },
}

#[derive(Clone, Debug)]
pub struct JwtProvider {
  pub issuer: Option<String>,
  pub audiences: HashSet<String>,
  pub optional_kid: bool,
  pub jwks: Jwks,
}

#[derive(Clone, Debug)]
pub enum AuthProvider {
  Basic(BasicProvider),
  Jwt(JwtProvider),
}

#[derive(Clone, Debug)]
pub struct AuthEntry {
  pub provider: AuthProvider,
}

#[derive(Clone, Default, Debug)]
pub struct Auth(pub Vec<AuthEntry>);

impl Auth {
  pub fn make(init_context: &InitContext, auth: &config::Auth) -> Valid<Auth, String> {
    Valid::from_iter(&auth.0, |input| {
      let provider = match &input.provider {
        config::AuthProvider::Basic(basic) => to_basic(init_context, basic.clone())
          .map(AuthProvider::Basic)
          .trace(config::Basic::directive_name().as_str()),
        config::AuthProvider::Jwt(jwt) => to_jwt(init_context, jwt.clone())
          .map(AuthProvider::Jwt)
          .trace(config::Jwt::directive_name().as_str()),
      };

      provider.map(|provider| AuthEntry { provider })
    })
    .map(Auth)
    .trace(config::Auth::directive_name().as_str())
  }
}

fn to_basic(init_context: &InitContext, options: config::Basic) -> Valid<BasicProvider, String> {
  match options {
    config::Basic::Htpasswd(data) => {
      Valid::from(Mustache::parse(&data).map_err(|e| ValidationError::new(e.to_string()))).map(|tmpl| {
        let htpasswd = tmpl.render(init_context);

        BasicProvider { htpasswd }
      })
    }
  }
}

fn to_jwt(init_context: &InitContext, options: config::Jwt) -> Valid<JwtProvider, String> {
  let jwks = &options.jwks;

  let jwks_valid = match &jwks {
    config::Jwks::Data(data) => Valid::from(Mustache::parse(data).map_err(|e| ValidationError::new(e.to_string())))
      .and_then(|tmpl| {
        let data = tmpl.render(init_context);

        if data.is_empty() {
          return Valid::fail("JWKS data is empty".into());
        }

        let de = &mut serde_json::Deserializer::from_str(&data);

        Valid::from(serde_path_to_error::deserialize(de).map_err(ValidationError::from))
          .map(|jwks: JwkSet| Jwks::Local(jwks))
      }),
    config::Jwks::Remote { url, max_age } => {
      Valid::from(Mustache::parse(url).map_err(|e| ValidationError::new(e.to_string()))).and_then(|url| {
        let url = url.render(init_context);

        Valid::from(Url::parse(&url).map_err(|e| ValidationError::new(e.to_string())))
          .map(|url| Jwks::Remote { url, max_age: Duration::from_millis(max_age.get()) })
      })
    }
  };

  jwks_valid.map(|jwks| JwtProvider {
    issuer: options.issuer,
    audiences: options.audiences,
    optional_kid: options.optional_kid,
    jwks,
  })
}
