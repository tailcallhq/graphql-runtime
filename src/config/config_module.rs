use std::ops::Deref;

use derive_setters::Setters;
use prost_reflect::prost_types::FileDescriptorSet;

use crate::config::Config;

/// A wrapper on top of Config that contains all the resolved extensions.
#[derive(Clone, Debug, Default, Setters)]
pub struct ConfigModule {
    pub config: Config,
    pub extensions: Extensions,
}

#[derive(Clone, Debug, Default)]
pub struct Content<A> {
    pub id: Option<String>,
    pub content: A,
}

impl<A> Deref for Content<A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

/// Extensions are meta-information required before we can generate the blueprint.
/// Typically, this information cannot be inferred without performing an IO operation, i.e.,
/// reading a file, making an HTTP call, etc.
#[derive(Clone, Debug, Default)]
pub struct Extensions {
    /// Contains the file descriptor sets resolved from the links
    pub grpc_file_descriptors: Vec<Content<FileDescriptorSet>>,

    /// Contains the contents of the JS file
    pub script: Option<String>,
}

impl Extensions {
    pub fn merge_right(mut self, other: &Extensions) -> Self {
        self.grpc_file_descriptors
            .extend(other.grpc_file_descriptors.clone());
        self.script = other.script.clone().or(self.script.take());
        self
    }

    pub fn get_file_descriptor(&self, id: &str) -> Option<&FileDescriptorSet> {
        self.grpc_file_descriptors
            .iter()
            .find(|content| content.id.as_deref() == Some(id))
            .map(|content| content.deref())
    }
}

impl ConfigModule {
    pub fn merge_right(mut self, other: &Self) -> Self {
        self.config = self.config.merge_right(&other.config);
        self.extensions = self.extensions.merge_right(&other.extensions);
        self
    }
}

impl Deref for ConfigModule {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl From<Config> for ConfigModule {
    fn from(config: Config) -> Self {
        ConfigModule { config, ..Default::default() }
    }
}
