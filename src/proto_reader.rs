use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use futures_util::future::join_all;
use prost_reflect::prost_types::{FileDescriptorProto, FileDescriptorSet};
use protox::file::{FileResolver, GoogleFileResolver};

use crate::resource_reader::ResourceReader;
use crate::runtime::TargetRuntime;

pub struct ProtoReader {
    resource_reader: ResourceReader,
    files: Vec<PathBuf>,
}

pub struct ProtoMetadata {
    pub descriptor_set: FileDescriptorSet,
    pub path: PathBuf,
}

impl ProtoReader {
    pub fn init<T: AsRef<Path>>(runtime: TargetRuntime, paths: &[T]) -> Self {
        Self {
            resource_reader: ResourceReader::init(runtime),
            files: paths.iter().map(|p| p.as_ref().to_path_buf()).collect(),
        }
    }

    // FIXME: rename to `load()`
    /// Performs BFS to import all nested proto files
    pub fn read_all(&self) -> anyhow::Result<Vec<ProtoMetadata>> {
        let includes: Vec<_> = self
            .files
            .iter()
            .filter_map(|path| path.ancestors().skip(1).next())
            .collect();

        Ok(protox::compile(&self.files, &includes)?
            .file
            .into_iter()
            .map(|proto_file| {
                let path = self
                    .files
                    .iter()
                    .find_map(|path| {
                        if path.ends_with(proto_file.name.as_ref().unwrap()) {
                            Some(path.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        includes
                            .iter()
                            .find_map(|path| {
                                let path = path
                                    .join(proto_file.name.as_ref()?);
                                if path.exists() {
                                    Some(path)
                                } else {
                                    None
                                }
                            })
                    })
                    .unwrap();
                ProtoMetadata {
                    descriptor_set: FileDescriptorSet {
                        file: vec![proto_file],
                    },
                    path
                }
            })
            .collect())
    }
    //
    // // FIXME: delete this method
    // pub async fn read<T: AsRef<str>>(&self, path: T) -> anyhow::Result<ProtoMetadata> {
    //     let proto = self.read_proto(path.as_ref()).await?;
    //     if proto.package.is_none() {
    //         anyhow::bail!("Package name is required");
    //     }
    //
    //     let descriptors = self.resolve_dependencies(proto).await?;
    //     let metadata = ProtoMetadata {
    //         descriptor_set: FileDescriptorSet { file: descriptors },
    //         path: path.as_ref().to_string(),
    //     };
    //     Ok(metadata)
    // }

    // FIXME: delete this method
    // Performs BFS to import all nested proto files
    // async fn resolve_dependencies(
    //     &self,
    //     parent_proto: FileDescriptorProto,
    // ) -> anyhow::Result<Vec<FileDescriptorProto>> {
    //     let mut queue = VecDeque::new();
    //     queue.push_back(parent_proto.clone());
    //
    //     while let Some(file) = queue.pop_front() {
    //         let futures: Vec<_> = file
    //             .dependency
    //             .iter()
    //             .map(|import| self.read_proto(import))
    //             .collect();
    //
    //         let results = join_all(futures).await;
    //
    //         for result in results {
    //             let proto = result?;
    //             let descriptors = self.descriptors.lock().unwrap();
    //             if descriptors.get(proto.name()).is_none() {
    //                 queue.push_back(proto.clone());
    //                 descriptors.insert(proto.name().to_string(), proto);
    //             }
    //         }
    //     }
    //
    //     let hash_map = self.descriptors.lock().unwrap();
    //
    //     let mut descriptors_vec = hash_map.into_values().collect::<Vec<FileDescriptorProto>>();
    //     descriptors_vec.push(parent_proto);
    //     Ok(descriptors_vec)
    // }


    // Tries to load well-known google proto files and if not found uses normal
    // file and http IO to resolve them
    // async fn read_proto(&self, path: impl AsRef<Path>) -> anyhow::Result<FileDescriptorProto> {
    //
    //     let content = if let Ok(file) = GoogleFileResolver::new().open_file(path) {
    //         file.source()
    //             .context("Unable to extract content of google well-known proto file")?
    //             .to_string()
    //     } else {
    //         self.resource_reader.read_file(path.as_ref().display()).await?.content
    //     };
    //     Ok(protox_parse::parse(path, &content)?)
    // }
}

#[cfg(test)]
mod test_proto_config {
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result};
    use pretty_assertions::assert_eq;

    use crate::proto_reader::ProtoReader;

    // #[tokio::test]
    // async fn test_resolve() {
    //     // Skipping IO tests as they are covered in reader.rs
    //     let reader = ProtoReader::init(crate::runtime::test::init(None));
    //     reader
    //         .read_proto("google/protobuf/empty.proto")
    //         .await
    //         .unwrap();
    // }

    // #[tokio::test]
    // async fn test_nested_imports() -> Result<()> {
    //     let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     let mut test_dir = root_dir.join(file!());
    //     test_dir.pop(); // src
    //
    //     let mut root = test_dir.clone();
    //     root.pop();
    //
    //     test_dir.push("grpc"); // grpc
    //     test_dir.push("tests"); // tests
    //     test_dir.push("proto"); // proto
    //
    //     let mut test_file = test_dir.clone();
    //
    //     test_file.push("nested0.proto"); // nested0.proto
    //     assert!(test_file.exists());
    //     let test_file = test_file.to_str().unwrap().to_string();
    //
    //     let runtime = crate::runtime::test::init(None);
    //     let file_rt = runtime.file.clone();
    //
    //     let reader = ProtoReader::init(runtime);
    //     let helper_map = reader
    //         .resolve_dependencies(reader.read_proto(&test_file).await?)
    //         .await?;
    //     let files = test_dir.read_dir()?;
    //     for file in files {
    //         let file = file?;
    //         let path = file.path();
    //         let path_str =
    //             path_to_file_name(path.as_path()).context("It must be able to extract path")?;
    //         let source = file_rt.read(&path_str).await?;
    //         let expected = protox_parse::parse(&path_str, &source)?;
    //         let actual = helper_map
    //             .iter()
    //             .find(|v| v.package.eq(&expected.package))
    //             .unwrap();
    //
    //         assert_eq!(&expected.dependency, &actual.dependency);
    //     }
    //
    //     Ok(())
    // }

    fn path_to_file_name(path: &Path) -> Option<String> {
        let components: Vec<_> = path.components().collect();

        // Find the index of the "src" component
        if let Some(src_index) = components.iter().position(|&c| c.as_os_str() == "src") {
            // Reconstruct the path from the "src" component onwards
            let after_src_components = &components[src_index..];
            let result = after_src_components
                .iter()
                .fold(PathBuf::new(), |mut acc, comp| {
                    acc.push(comp);
                    acc
                });
            Some(result.to_str().unwrap().to_string())
        } else {
            None
        }
    }
    // #[tokio::test]
    // async fn test_proto_no_pkg() -> Result<()> {
    //     let runtime = crate::runtime::test::init(None);
    //     let reader = ProtoReader::init(runtime);
    //     let mut proto_no_pkg = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     proto_no_pkg.push("src/grpc/tests/proto_no_pkg.graphql");
    //     let config_module = reader.read(proto_no_pkg.to_str().unwrap()).await;
    //     assert!(config_module.is_err());
    //     Ok(())
    // }
}
