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
    extra_scrapers::*, // Importa todos los nuevos
};

/// 🏭 FÁBRICA CENTRAL DE SCRAPERS
/// Devuelve una lista con TODOS los agentes de scraping disponibles.
/// Si añades un nuevo scraper, regístralo aquí.
pub fn get_all_scrapers() -> Vec<Arc<dyn Agent>> {
    vec![
        // Scrapers Clásicos
        Arc::new(RemoteOkAgent::new()),
        Arc::new(ArbeitnowAgent::new()),
        Arc::new(HimalayasAgent::new()),
        Arc::new(WwrAgent::new()),
        Arc::new(JobspressoAgent::new()),

        // Scrapers Extra (Nuevos)
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
    ]
}
