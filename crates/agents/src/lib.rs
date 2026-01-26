pub mod analyzer;
pub mod enricher;
pub mod scrapers;

use std::sync::Arc;
use job_hunter_core::Agent;

pub use crate::analyzer::{AnalyzerAgent, UseCase};
pub use crate::enricher::EnricherAgent;

// Importamos todos los módulos de scrapers
use crate::scrapers::{
    arbeitnow::ArbeitnowAgent,
    himalayas::HimalayasAgent,
    jobspresso::JobspressoAgent,
    remoteok::RemoteOkAgent,
    weworkremotely::WwrAgent,
    extra_scrapers::*, 
    // Si quieres usar el dynamic, descomenta abajo:
    // dynamic::DynamicScraperAgent,
};

// Necesitas añadir esto al mod scrapers/mod.rs también: pub mod dynamic;

pub fn get_all_scrapers() -> Vec<Arc<dyn Agent>> {
    vec![
        Arc::new(RemoteOkAgent::new()),
        Arc::new(ArbeitnowAgent::new()),
        Arc::new(HimalayasAgent::new()),
        Arc::new(WwrAgent::new()),
        Arc::new(JobspressoAgent::new()),

        Arc::new(RemotiveAgent::new()),
        Arc::new(JobicyAgent::new()),
        Arc::new(FindWorkAgent::new()),
        Arc::new(WorkingNomadsAgent::new()),
        Arc::new(VueJobsAgent::new()),
        Arc::new(CryptoJobsAgent::new()),
        Arc::new(DevItJobsAgent::new()),
        Arc::new(GolangProjectsAgent::new()),
        Arc::new(PythonOrgAgent::new()),
        Arc::new(RemoteCoAgent::new()),
        
        // Arc::new(DynamicScraperAgent::new()), 
    ]
}
