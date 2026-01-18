//! Vagueness detection heuristics for adaptive planning mode.
//!
//! Analyzes user input to determine if it's specific enough for direct planning
//! or requires clarifying questions.

/// Score indicating how vague a user input is.
#[derive(Debug, Clone)]
pub struct VaguenessScore {
    /// Score from 0.0 (very specific) to 1.0 (very vague).
    pub score: f32,
    /// Reasons explaining why the input is considered vague/specific.
    pub reasons: Vec<String>,
}

impl VaguenessScore {
    /// Returns true if the score exceeds the vagueness threshold.
    ///
    /// Threshold is 0.5 - inputs above this trigger clarification questions.
    pub fn is_vague(&self) -> bool {
        self.score > 0.5
    }
}

/// Assess how vague a user input is.
///
/// Returns a score from 0.0 (very specific) to 1.0 (very vague) along with
/// reasons explaining the assessment.
///
/// # Scoring Heuristics
///
/// 1. Word count analysis:
///    - < 5 words: +0.55 (Very short input)
///    - 5-15 words: +0.2 (Short input)
///    - > 15 words: no change
///
/// 2. Specificity markers (any present): -0.2
///    - Action verbs: "must", "should", "requires", "needs to"
///    - Implementation: "using", "with", "implement", "add", "create"
///    - Technical: "endpoint", "api", "database", "component", "module", "function", "class"
///
/// 3. Vagueness markers (+0.15 each):
///    - Indefinite: "something", "somehow", "stuff", "things", "whatever"
///    - Uncertain: "maybe", "possibly", "kind of", "sort of", "like a", "basically"
///
/// 4. Question handling:
///    - Short question (< 10 words with "?"): +0.2
pub fn assess_vagueness(input: &str) -> VaguenessScore {
    let mut score = 0.0f32;
    let mut reasons = Vec::new();

    let input_lower = input.to_lowercase();
    let words: Vec<&str> = input.split_whitespace().collect();
    let word_count = words.len();

    // 1. Word count analysis
    if word_count < 5 {
        score += 0.55;
        reasons.push("Very short input".to_string());
    } else if word_count <= 15 {
        score += 0.2;
        reasons.push("Short input".to_string());
    }

    // 2. Specificity markers (REDUCE vagueness if ANY present)
    let action_verbs = ["must", "should", "requires", "needs to"];
    let implementation = ["using", "with", "implement", "add", "create"];
    let technical = [
        "endpoint", "api", "database", "component", "module", "function", "class",
    ];

    let has_action = action_verbs.iter().any(|&m| input_lower.contains(m));
    let has_impl = implementation.iter().any(|&m| input_lower.contains(m));
    let has_technical = technical.iter().any(|&m| input_lower.contains(m));

    if has_action || has_impl || has_technical {
        score -= 0.2;
        reasons.push("Has specificity markers".to_string());
    }

    // 3. Vagueness markers (INCREASE vagueness +0.15 each)
    let indefinite = ["something", "somehow", "stuff", "things", "whatever"];
    let uncertain = [
        "maybe",
        "possibly",
        "kind of",
        "sort of",
        "like a",
        "basically",
    ];

    for marker in indefinite.iter() {
        if input_lower.contains(marker) {
            score += 0.15;
            reasons.push(format!("Indefinite term: '{}'", marker));
        }
    }

    for marker in uncertain.iter() {
        if input_lower.contains(marker) {
            score += 0.15;
            reasons.push(format!("Uncertain term: '{}'", marker));
        }
    }

    // 4. Question handling
    if input.contains('?') && word_count < 10 {
        score += 0.2;
        reasons.push("Short question".to_string());
    }

    // Clamp to 0.0..1.0
    score = score.clamp(0.0, 1.0);

    VaguenessScore { score, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_input_low_score() {
        let result =
            assess_vagueness("Add a REST API endpoint for user authentication using JWT tokens");
        assert!(
            result.score < 0.3,
            "Specific input should have low score: {}",
            result.score
        );
        assert!(
            !result.is_vague(),
            "Specific input should not be vague: {:?}",
            result
        );
    }

    #[test]
    fn test_vague_input_high_score() {
        let result = assess_vagueness("make something for stuff");
        assert!(
            result.score > 0.7,
            "Vague input should have high score: {}",
            result.score
        );
        assert!(
            result.is_vague(),
            "Vague input should be vague: {:?}",
            result
        );
    }

    #[test]
    fn test_short_input_vague() {
        let result = assess_vagueness("todo app");
        assert!(
            result.score > 0.5,
            "Short input should be vague: {}",
            result.score
        );
        assert!(result.is_vague());
    }

    #[test]
    fn test_medium_input_with_details_not_vague() {
        let result = assess_vagueness("Build a todo app with React and PostgreSQL");
        assert!(
            result.score < 0.5,
            "Medium detailed input should not be too vague: {}",
            result.score
        );
        assert!(
            !result.is_vague(),
            "Detailed medium input should not be vague"
        );
    }

    #[test]
    fn test_short_question_vague() {
        let result = assess_vagueness("how to do this?");
        assert!(
            result.reasons.contains(&"Short question".to_string()),
            "Should detect short question"
        );
        assert!(result.is_vague());
    }

    #[test]
    fn test_reasons_populated() {
        let result = assess_vagueness("maybe something");
        assert!(
            !result.reasons.is_empty(),
            "Reasons should be populated: {:?}",
            result
        );
    }

    #[test]
    fn test_score_clamped() {
        // Very vague input with multiple markers
        let result =
            assess_vagueness("maybe something somehow with stuff and things and whatever basically");
        assert!(
            result.score <= 1.0,
            "Score should be clamped to 1.0: {}",
            result.score
        );
        assert!(result.score >= 0.0, "Score should be >= 0.0");
    }

    #[test]
    fn test_specificity_reduces_score() {
        // Short but specific
        let result_with = assess_vagueness("create database module");
        let result_without = assess_vagueness("make db thing");

        assert!(
            result_with.score < result_without.score,
            "Specificity markers should reduce score: {} vs {}",
            result_with.score,
            result_without.score
        );
    }

    #[test]
    fn test_jwt_auth_endpoint_not_vague() {
        let result = assess_vagueness("Add JWT auth endpoint using jose library");
        assert!(
            !result.is_vague(),
            "Technical endpoint description should not be vague: score = {}",
            result.score
        );
    }
}
