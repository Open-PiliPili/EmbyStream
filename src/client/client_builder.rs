use crate::network::NetworkPlugin;

// Generic ClientBuilder for any client type that can be built from plugins
pub struct ClientBuilder<T> {
    plugins: Vec<Box<dyn NetworkPlugin>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Default for ClientBuilder<T>
where
    T: BuildableClient,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ClientBuilder<T>
where
    T: BuildableClient,
{
    /// Creates a new builder with default configuration.
    ///
    /// Starts with an empty set of network plugins. You'll typically want to add
    /// at least one network implementation like `CurlPlugin`.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Adds a network plugin to the client's configuration.
    ///
    /// # Arguments
    /// - `plugin`: Network plugin implementing the transport layer.
    ///
    /// # Note
    /// Plugins are used in the order they're added. The first compatible plugin
    /// will handle each request.
    pub fn with_plugin(mut self, plugin: impl NetworkPlugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Constructs the client with the configured plugins.
    ///
    /// Consumes the builder and returns the finalized client instance.
    ///
    /// # Returns
    /// The constructed client of type `T`.
    pub fn build(self) -> T {
        T::build_from_plugins(self.plugins)
    }
}

/// Trait for clients that can be built from a list of plugins.
pub trait BuildableClient {
    fn build_from_plugins(plugins: Vec<Box<dyn NetworkPlugin>>) -> Self;
}
