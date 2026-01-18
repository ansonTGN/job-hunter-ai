// Ahora analyzer es un módulo (carpeta)
pub mod analyzer; 
pub mod enricher;
pub mod scrapers;

pub use crate::analyzer::{AnalyzerAgent, UseCase};
pub use crate::enricher::EnricherAgent;

pub use crate::scrapers::arbeitnow::ArbeitnowAgent;
pub use crate::scrapers::himalayas::HimalayasAgent;
pub use crate::scrapers::jobspresso::JobspressoAgent;
pub use crate::scrapers::remoteok::RemoteOkAgent;
pub use crate::scrapers::weworkremotely::WwrAgent;
