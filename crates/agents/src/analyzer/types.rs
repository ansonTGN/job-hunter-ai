use job_hunter_core::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UseCase {
    Fast,
    Balanced,
    Deep,
    LongContext,
}

impl UseCase {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fast" | "rapido" | "rápido" => Self::Fast,
            "deep" | "profundo" => Self::Deep,
            "long" | "long_context" | "contexto_largo" => Self::LongContext,
            _ => Self::Balanced,
        }
    }
}

#[derive(Clone)]
pub enum LlmProvider {
    OpenAI {
        api_key: String,
        base_url: String,
        model: Option<String>,
        use_case: UseCase,
    },
    Anthropic {
        api_key: String,
        base_url: String,
        model: Option<String>,
        use_case: UseCase,
        version: String,
    },
    Local {
        endpoint: String,
        model: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TolerantSkillsGap {
    #[serde(default)]
    pub matching: Vec<String>,
    #[serde(default)]
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmAnalysis {
    pub title: Option<String>,
    pub company: Option<CompanyInfo>,
    pub company_name: Option<String>,
    pub description: Option<String>,
    pub salary_normalized: Option<f64>,
    pub red_flags: Option<Vec<String>>,
    pub skills_analysis: Option<TolerantSkillsGap>,
    pub requirements: Option<Vec<String>>,
    pub responsibilities: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub location: Option<String>,
    pub is_remote: Option<bool>,
    pub job_type: Option<String>,
    pub experience_level: Option<String>,
    pub match_score: Option<f32>,
    pub match_reasons: Option<Vec<String>>,
}

impl LlmAnalysis {
    pub fn into_analyzed(self, raw: &RawJobPosting, criteria: &SearchCriteria) -> AnalyzedJobPosting {
        let company = match (self.company, self.company_name) {
            (Some(c), _) => Some(c),
            (None, Some(name)) if !name.trim().is_empty() => Some(CompanyInfo {
                name,
                description: None,
                industry: None,
                size: None,
                website: None,
                linkedin_url: None,
            }),
            _ => None,
        };

        let job_type = parse_job_type(self.job_type.as_deref()).unwrap_or(JobType::FullTime);
        let exp = parse_experience(self.experience_level.as_deref())
            .unwrap_or(criteria.experience_level.clone());

        let match_score = self.match_score.unwrap_or(0.0).clamp(0.0, 1.0);

        let skills_analysis = self.skills_analysis.map(|t| SkillsGap {
            matching: t.matching,
            missing: t.missing,
        }).unwrap_or_default();

        AnalyzedJobPosting {
            id: raw.id.clone(),
            title: self.title.unwrap_or_else(|| "(sin título)".to_string()),
            company,
            description: self.description.unwrap_or_else(|| "".to_string()),
            salary_normalized: self.salary_normalized,
            red_flags: self.red_flags.unwrap_or_default(),
            skills_analysis,
            requirements: self.requirements.unwrap_or_default(),
            responsibilities: self.responsibilities.unwrap_or_default(),
            skills: self.skills.unwrap_or_default(),
            salary_range: None,
            location: self.location.unwrap_or_else(|| "Remote".to_string()),
            is_remote: self.is_remote.unwrap_or(true),
            job_type,
            experience_level: exp,
            url: raw.url.clone(),
            posted_date: None,
            match_score,
            match_reasons: self.match_reasons.unwrap_or_default(),
        }
    }
}

fn parse_job_type(s: Option<&str>) -> Option<JobType> {
    let v = s?.trim().to_lowercase();
    match v.as_str() {
        "fulltime" | "full_time" | "full-time" => Some(JobType::FullTime),
        "parttime" | "part_time" | "part-time" => Some(JobType::PartTime),
        "contract" => Some(JobType::Contract),
        "freelance" => Some(JobType::Freelance),
        "internship" => Some(JobType::Internship),
        _ => None,
    }
}

fn parse_experience(s: Option<&str>) -> Option<ExperienceLevel> {
    let v = s?.trim().to_lowercase();
    match v.as_str() {
        "entry" => Some(ExperienceLevel::Entry),
        "junior" => Some(ExperienceLevel::Junior),
        "mid" | "middle" => Some(ExperienceLevel::Mid),
        "senior" => Some(ExperienceLevel::Senior),
        "lead" => Some(ExperienceLevel::Lead),
        "any" => Some(ExperienceLevel::Any),
        _ => None,
    }
}