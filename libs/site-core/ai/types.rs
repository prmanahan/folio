use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Deserialize)]
pub struct FitRequest {
    pub job_description: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitVerdict {
    pub verdict: String,
    pub headline: String,
    pub opening: String,
    pub gaps: Vec<FitGap>,
    pub transfers: Vec<FitTransfer>,
    pub recommendation: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitGap {
    pub requirement: String,
    pub gap_title: String,
    pub explanation: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitTransfer {
    pub skill: String,
    pub relevance: String,
}
