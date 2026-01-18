use console::style;
use job_hunter_core::AnalyzedJobPosting;

pub struct ConsoleReporter;

impl ConsoleReporter {
    pub fn print_banner() {
        println!("{}", style("🚀 Job Hunter System").bold().cyan());
    }

    pub fn print_job_card(job: &AnalyzedJobPosting, idx: usize) {
        let score_style = if job.match_score > 0.8 {
            style(format!("{:.0}%", job.match_score * 100.0)).green()
        } else {
            style(format!("{:.0}%", job.match_score * 100.0)).yellow()
        };

        println!("\n{}. {} [{}]", idx, style(&job.title).bold(), score_style);
        if let Some(comp) = &job.company {
            println!("   🏢 {}", comp.name);
        }
        println!("   🔗 {}", style(&job.url).underlined().blue());
    }
}
