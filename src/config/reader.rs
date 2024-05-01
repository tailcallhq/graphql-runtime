use std::path::Path;
use std::sync::Arc;

use rustls_pemfile;
use rustls_pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer,
};

use super::{ConfigModule, Content, Link, LinkType};
use crate::config::{Config, ConfigReaderContext, Source};
use crate::merge_right::MergeRight;
use crate::proto_reader::ProtoReader;
use crate::resource_reader::{Cached, ResourceReader};
use crate::rest::EndpointSet;
use crate::runtime::TargetRuntime;

/// Reads the configuration from a file or from an HTTP URL and resolves all
/// linked extensions to create a ConfigModule.
pub struct ConfigReader {
    runtime: TargetRuntime,
    resource_reader: ResourceReader<Cached>,
    proto_reader: ProtoReader,
}

impl ConfigReader {
    pub fn init(runtime: TargetRuntime) -> Self {
        let resource_reader = ResourceReader::<Cached>::cached(runtime.clone());
        Self {
            runtime: runtime.clone(),
            resource_reader: resource_reader.clone(),
            proto_reader: ProtoReader::init(resource_reader, runtime),
        }
    }

    /// Reads the links in a Config and fill the content
    #[async_recursion::async_recursion]
    async fn ext_links<'a: 'async_recursion>(
        &self,
        mut config_module: ConfigModule,
        parent_dir: Option<&'a Path>,
    ) -> anyhow::Result<ConfigModule> {
        let links: Vec<Link> = config_module
            .config
            .links
            .clone()
            .iter()
            .filter_map(|link| {
                if link.src.is_empty() {
                    return None;
                }
                Some(link.to_owned())
            })
            .collect();

        if links.is_empty() {
            return Ok(config_module);
        }

        for link in links.iter() {
            let path = Self::resolve_path(&link.src, parent_dir);

            match link.type_of {
                LinkType::Config => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;

                    let config = Config::from_source(Source::detect(&source.path)?, &content)?;

                    config_module = config_module.merge_right(ConfigModule::from(config.clone()));

                    if !config.links.is_empty() {
                        config_module = config_module.merge_right(
                            self.ext_links(
                                ConfigModule::from(config),
                                Path::new(&link.src).parent(),
                            )
                            .await?,
                        );
                    }
                }
                LinkType::Protobuf => {
                    let meta = self.proto_reader.read(path).await?;
                    config_module.extensions.add_proto(meta);
                }
                LinkType::Script => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;
                    config_module.extensions.script = Some(content);
                }
                LinkType::Cert => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;
                    config_module
                        .extensions
                        .cert
                        .extend(self.load_cert(content).await?);
                }
                LinkType::Key => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;
                    config_module.extensions.keys = Arc::new(self.load_private_key(content).await?)
                }
                LinkType::Operation => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;

                    config_module.extensions.endpoint_set = EndpointSet::try_new(&content)?;
                }
                LinkType::Htpasswd => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;

                    config_module
                        .extensions
                        .htpasswd
                        .push(Content { id: link.id.clone(), content });
                }
                LinkType::Jwks => {
                    let source = self.resource_reader.read_file(&path).await?;
                    let content = source.content;

                    let de = &mut serde_json::Deserializer::from_str(&content);

                    config_module.extensions.jwks.push(Content {
                        id: link.id.clone(),
                        content: serde_path_to_error::deserialize(de)?,
                    })
                }
                LinkType::Grpc => {
                    let meta = self.proto_reader.fetch(link.src.as_str()).await?;

                    for m in meta {
                        config_module.extensions.add_proto(m);
                    }
                }
            }
        }

        Ok(config_module)
    }

    /// Reads the certificate from a given file
    async fn load_cert(&self, content: String) -> anyhow::Result<Vec<CertificateDer<'static>>> {
        let certificates = rustls_pemfile::certs(&mut content.as_bytes())?;

        Ok(certificates.into_iter().map(CertificateDer::from).collect())
    }

    /// Reads a private key from a given file
    async fn load_private_key(
        &self,
        content: String,
    ) -> anyhow::Result<Vec<PrivateKeyDer<'static>>> {
        let keys = rustls_pemfile::read_all(&mut content.as_bytes())?;

        Ok(keys
            .into_iter()
            .filter_map(|key| match key {
                rustls_pemfile::Item::RSAKey(key) => {
                    Some(PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key)))
                }
                rustls_pemfile::Item::ECKey(key) => {
                    Some(PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(key)))
                }
                rustls_pemfile::Item::PKCS8Key(key) => {
                    Some(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)))
                }
                _ => None,
            })
            .collect())
    }

    /// Reads a single file and returns the config
    pub async fn read<T: ToString + Send + Sync>(&self, file: T) -> anyhow::Result<ConfigModule> {
        self.read_all(&[file]).await
    }

    /// Reads all the files and returns a merged config
    pub async fn read_all<T: ToString + Send + Sync>(
        &self,
        files: &[T],
    ) -> anyhow::Result<ConfigModule> {
        let files = self.resource_reader.read_files(files).await?;
        let mut config_module = ConfigModule::default();

        for file in files.iter() {
            let source = Source::detect(&file.path)?;
            let schema = &file.content;

            // Create initial config module
            let new_config_module = self
                .resolve(
                    Config::from_source(source, schema)?,
                    Path::new(&file.path).parent(),
                )
                .await?;

            // Merge it with the original config set
            config_module = config_module.merge_right(new_config_module);
        }

        Ok(config_module)
    }

    /// Resolves all the links in a Config to create a ConfigModule
    pub async fn resolve(
        &self,
        config: Config,
        parent_dir: Option<&Path>,
    ) -> anyhow::Result<ConfigModule> {
        // Create initial config set
        let config_module = ConfigModule::from(config);

        // Extend it with the links
        let mut config_module = self.ext_links(config_module, parent_dir).await?;

        let server = &mut config_module.config.server;
        let reader_ctx = ConfigReaderContext {
            runtime: &self.runtime,
            vars: &server
                .vars
                .iter()
                .map(|vars| (vars.key.clone(), vars.value.clone()))
                .collect(),
            headers: Default::default(),
        };

        config_module
            .config
            .telemetry
            .render_mustache(&reader_ctx)?;

        Ok(config_module)
    }

    /// Checks if path is absolute else it joins file path with relative dir
    /// path
    fn resolve_path(src: &str, root_dir: Option<&Path>) -> String {
        if Path::new(&src).is_absolute() {
            src.to_string()
        } else {
            let path = root_dir.unwrap_or(Path::new(""));
            path.join(src).to_string_lossy().to_string()
        }
    }
}

#[cfg(test)]
mod reader_tests {
    use std::path::{Path, PathBuf};

    use pretty_assertions::assert_eq;

    use crate::config::reader::ConfigReader;
    use crate::config::{Config, Type};

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    #[tokio::test]
    async fn test_all() {
        let runtime = crate::runtime::test::init(None);

        let mut cfg = Config::default();
        cfg.schema.query = Some("Test".to_string());
        cfg = cfg.types([("Test", Type::default())].to_vec());

        let server = start_mock_server();
        let header_server = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/bar.graphql");
            then.status(200).body(cfg.to_sdl());
        });

        let json = runtime
            .file
            .read("examples/jsonplaceholder.json")
            .await
            .unwrap();

        let foo_json_server = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/foo.json");
            then.status(200).body(json);
        });

        let port = server.port();
        let files: Vec<String> = [
            "examples/jsonplaceholder.yml", // config from local file
            format!("http://localhost:{port}/bar.graphql").as_str(), // with content-type header
            format!("http://localhost:{port}/foo.json").as_str(), // with url extension
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();
        let cr = ConfigReader::init(runtime);
        let c = cr.read_all(&files).await.unwrap();
        assert_eq!(
            ["Post", "Query", "Test", "User"]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>(),
            c.types
                .keys()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
        );
        foo_json_server.assert(); // checks if the request was actually made
        header_server.assert();
    }

    #[tokio::test]
    async fn test_local_files() {
        let runtime = crate::runtime::test::init(None);

        let files: Vec<String> = [
            "examples/jsonplaceholder.yml",
            "examples/jsonplaceholder.graphql",
            "examples/jsonplaceholder.json",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();
        let cr = ConfigReader::init(runtime);
        let c = cr.read_all(&files).await.unwrap();
        assert_eq!(
            ["Post", "Query", "User"]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>(),
            c.types
                .keys()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
        );
    }

    #[tokio::test]
    async fn test_script_loader() {
        let runtime = crate::runtime::test::init(None);
        let file_rt = runtime.file.clone();

        let cargo_manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let reader = ConfigReader::init(runtime);

        let config = reader
            .read(&format!(
                "{}/examples/jsonplaceholder_script.graphql",
                cargo_manifest
            ))
            .await
            .unwrap();

        let path = format!("{}/examples/scripts/echo.js", cargo_manifest);
        let content = file_rt.read(&path).await;

        assert_eq!(content.unwrap(), config.extensions.script.unwrap());
    }

    #[test]
    fn test_relative_path() {
        let path_dir = Path::new("abc/xyz");
        let file_relative = "foo/bar/my.proto";
        let file_absolute = "/foo/bar/my.proto";
        assert_eq!(
            path_dir.to_path_buf().join(file_relative),
            PathBuf::from(ConfigReader::resolve_path(file_relative, Some(path_dir)))
        );
        assert_eq!(
            "/foo/bar/my.proto",
            ConfigReader::resolve_path(file_absolute, Some(path_dir))
        );
    }
}
