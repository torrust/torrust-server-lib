//! Runtime service registry.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use torrust_net_primitives::service_binding::ServiceBinding;

/// A [`ServiceHeathCheckResult`] is returned by a completed health check.
pub type ServiceHeathCheckResult = Result<String, String>;

/// The [`ServiceHealthCheckJob`] has a health check job with it's metadata
///
/// The `job` awaits a [`ServiceHeathCheckResult`].
#[derive(Debug)]
pub struct ServiceHealthCheckJob {
    pub info: String,
    pub job: JoinHandle<ServiceHeathCheckResult>,
}

impl ServiceHealthCheckJob {
    #[must_use]
    pub fn new(info: String, job: JoinHandle<ServiceHeathCheckResult>) -> Self {
        Self { info, job }
    }
}

/// The function specification [`FnSpawnServiceHeathCheck`].
///
/// A function fulfilling this specification will spawn a new [`ServiceHealthCheckJob`].
pub type FnSpawnServiceHeathCheck = fn(&ServiceBinding) -> ServiceHealthCheckJob;

/// Immutable data reported by a started local service.
///
/// Metadata belongs to the application that uses the registry. The registry
/// does not assign semantics to it.
#[derive(Clone, Debug)]
pub struct ServiceRegistration<M> {
    service_binding: ServiceBinding,
    metadata: M,
    health_check: Option<FnSpawnServiceHeathCheck>,
}

impl<M> ServiceRegistration<M> {
    #[must_use]
    pub fn new(service_binding: ServiceBinding, metadata: M, health_check: Option<FnSpawnServiceHeathCheck>) -> Self {
        Self {
            service_binding,
            metadata,
            health_check,
        }
    }

    #[must_use]
    pub fn service_binding(&self) -> &ServiceBinding {
        &self.service_binding
    }

    #[must_use]
    pub fn metadata(&self) -> &M {
        &self.metadata
    }

    #[must_use]
    pub fn spawn_check(&self) -> Option<ServiceHealthCheckJob> {
        self.health_check.map(|health_check| health_check(&self.service_binding))
    }
}

/// A cloneable, immutable view of a registered service.
#[derive(Clone, Debug)]
pub struct RegisteredService<M> {
    registration: ServiceRegistration<M>,
}

impl<M> RegisteredService<M> {
    #[must_use]
    pub fn service_binding(&self) -> &ServiceBinding {
        self.registration.service_binding()
    }

    #[must_use]
    pub fn metadata(&self) -> &M {
        self.registration.metadata()
    }

    #[must_use]
    pub fn spawn_check(&self) -> Option<ServiceHealthCheckJob> {
        self.registration.spawn_check()
    }
}

/// Registration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    /// A service already owns this final local binding.
    DuplicateBinding(ServiceBinding),
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBinding(service_binding) => {
                write!(formatter, "a service is already registered for binding {service_binding}")
            }
        }
    }
}

impl std::error::Error for RegistrationError {}

/// A single-use registration capability for one started service.
///
/// Obtain one form per successfully bound service with
/// [`Registar::give_form`]. Consuming [`Self::register`] acknowledges that the
/// registration is visible in registry snapshots.
#[derive(Debug)]
pub struct ServiceRegistrationForm<M> {
    registar: Registar<M>,
}

impl<M> ServiceRegistrationForm<M> {
    /// Inserts a registration and returns only after it is visible to queries.
    ///
    /// A caller may treat successful completion as its registry-readiness
    /// acknowledgement after binding its listener.
    ///
    /// # Errors
    ///
    /// Returns [`RegistrationError::DuplicateBinding`] when another service is
    /// already registered with the same final binding.
    pub async fn register(self, registration: ServiceRegistration<M>) -> Result<(), RegistrationError> {
        self.registar.insert(registration).await
    }
}

/// The [`Registar`] manages immutable runtime service registrations.
#[derive(Debug)]
pub struct Registar<M = ()> {
    registry: Arc<Mutex<HashMap<ServiceBinding, ServiceRegistration<M>>>>,
}

impl<M> Clone for Registar<M> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
        }
    }
}

impl<M> Default for Registar<M> {
    fn default() -> Self {
        Self {
            registry: Arc::default(),
        }
    }
}

impl<M> Registar<M> {
    /// Returns a capability to register one service.
    #[must_use]
    pub fn give_form(&self) -> ServiceRegistrationForm<M> {
        ServiceRegistrationForm { registar: self.clone() }
    }

    async fn insert(&self, service_registration: ServiceRegistration<M>) -> Result<(), RegistrationError> {
        let mut mutex = self.registry.lock().await;

        if mutex.contains_key(service_registration.service_binding()) {
            return Err(RegistrationError::DuplicateBinding(
                service_registration.service_binding().clone(),
            ));
        }

        mutex.insert(service_registration.service_binding.clone(), service_registration);

        Ok(())
    }

    /// Returns a deterministic, side-effect-free snapshot of all services.
    ///
    /// Results are ordered by protocol, then final socket address, never by
    /// insertion or hash-map iteration order.
    pub async fn services(&self) -> Vec<RegisteredService<M>>
    where
        M: Clone,
    {
        let mutex = self.registry.lock().await;
        let mut services: Vec<_> = mutex
            .values()
            .cloned()
            .map(|registration| RegisteredService { registration })
            .collect();
        services.sort_by(|left, right| {
            protocol_sort_key(&left.service_binding().protocol())
                .cmp(&protocol_sort_key(&right.service_binding().protocol()))
                .then_with(|| {
                    left.service_binding()
                        .bind_address()
                        .cmp(&right.service_binding().bind_address())
                })
        });
        services
    }

    /// Returns a deterministic, side-effect-free metadata query result.
    pub async fn services_matching<F>(&self, predicate: F) -> Vec<RegisteredService<M>>
    where
        M: Clone,
        F: Fn(&M) -> bool,
    {
        self.services()
            .await
            .into_iter()
            .filter(|service| predicate(service.metadata()))
            .collect()
    }
}

fn protocol_sort_key(protocol: &torrust_net_primitives::service_binding::Protocol) -> u8 {
    match protocol {
        torrust_net_primitives::service_binding::Protocol::UDP => 0,
        torrust_net_primitives::service_binding::Protocol::HTTP => 1,
        torrust_net_primitives::service_binding::Protocol::HTTPS => 2,
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use torrust_net_primitives::service_binding::Protocol;

    use super::{Registar, RegistrationError, ServiceRegistration};

    fn binding(protocol: Protocol, port: u16) -> torrust_net_primitives::service_binding::ServiceBinding {
        torrust_net_primitives::service_binding::ServiceBinding::new(protocol, SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
            .expect("test binding should be valid")
    }

    #[tokio::test]
    async fn it_should_make_a_registration_visible_after_acknowledgement() {
        let registar = Registar::default();

        registar
            .give_form()
            .register(ServiceRegistration::new(binding(Protocol::HTTP, 8000), "first", None))
            .await
            .expect("registration should succeed");

        assert_eq!(registar.services().await[0].metadata(), &"first");
    }

    #[tokio::test]
    async fn it_should_return_services_in_deterministic_binding_order() {
        let registar = Registar::default();

        registar
            .give_form()
            .register(ServiceRegistration::new(binding(Protocol::HTTP, 9000), "second", None))
            .await
            .expect("registration should succeed");
        registar
            .give_form()
            .register(ServiceRegistration::new(binding(Protocol::HTTP, 8000), "first", None))
            .await
            .expect("registration should succeed");

        let metadata: Vec<_> = registar
            .services()
            .await
            .into_iter()
            .map(|service| *service.metadata())
            .collect();

        assert_eq!(metadata, ["first", "second"]);
    }

    #[tokio::test]
    async fn it_should_order_services_by_protocol_then_final_binding() {
        let registar = Registar::default();

        for (protocol, port, metadata) in [
            (Protocol::HTTPS, 8000, "https"),
            (Protocol::HTTP, 9000, "http-second"),
            (Protocol::UDP, 9000, "udp-second"),
            (Protocol::HTTP, 8000, "http-first"),
            (Protocol::UDP, 8000, "udp-first"),
        ] {
            registar
                .give_form()
                .register(ServiceRegistration::new(binding(protocol, port), metadata, None))
                .await
                .expect("registration should succeed");
        }

        let metadata: Vec<_> = registar
            .services()
            .await
            .into_iter()
            .map(|service| *service.metadata())
            .collect();

        assert_eq!(metadata, ["udp-first", "udp-second", "http-first", "http-second", "https"]);
    }

    #[tokio::test]
    async fn it_should_reject_duplicate_final_bindings() {
        let registar = Registar::default();
        let service_binding = binding(Protocol::HTTP, 8000);

        registar
            .give_form()
            .register(ServiceRegistration::new(service_binding.clone(), (), None))
            .await
            .expect("initial registration should succeed");

        let error = registar
            .give_form()
            .register(ServiceRegistration::new(service_binding.clone(), (), None))
            .await
            .expect_err("duplicate registration should fail");

        assert_eq!(error, RegistrationError::DuplicateBinding(service_binding));
    }
}
