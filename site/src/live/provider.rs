//! The model providers a visitor can choose, and the models each one offers.

/// Whose API answers the visitor. One key for one provider at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Provider {
    #[default]
    Anthropic,
    OpenAi,
}

#[allow(
    dead_code,
    reason = "the page uses these once it has a provider picker"
)]
impl Provider {
    pub const ALL: [Self; 2] = [Self::Anthropic, Self::OpenAi];

    pub fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
        }
    }

    /// The provider with the id `id`, if there is one.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|provider| provider.id() == id)
    }

    /// Where a visitor creates an API key for this provider.
    pub fn keys_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://platform.claude.com/settings/keys",
            // From the "Create and export an API key" step of
            // https://developers.openai.com/api/docs/quickstart (2026-11-24).
            Self::OpenAi => "https://platform.openai.com/api-keys",
        }
    }

    /// The model picked when the visitor switches to this provider.
    pub fn default_model(self) -> Model {
        match self {
            Self::Anthropic => Model::Sonnet,
            Self::OpenAi => Model::Sol,
        }
    }
}

/// The models a visitor can pick. Anything else from the browser is rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Model {
    Opus,
    #[default]
    Sonnet,
    Haiku,
    /// The OpenAI models are copied from https://developers.openai.com/api/docs/models
    /// as of 2026-11-24. Each one lists function calling, reasoning and the
    /// Responses API. Astra is the flagship for complex reasoning.
    Astra,
    /// The model OpenAI's page offers to "balance intelligence and cost": the
    /// mid-tier general model, and the OpenAI default here.
    Sol,
    /// The cost-sensitive, high-volume model.
    Luna,
}

impl Model {
    pub const ALL: [Self; 6] = [
        Self::Opus,
        Self::Sonnet,
        Self::Haiku,
        Self::Astra,
        Self::Sol,
        Self::Luna,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Opus => "claude-opus-5-5",
            Self::Sonnet => "claude-sonnet-5-5",
            Self::Haiku => "claude-haiku-5-5",
            Self::Astra => "gpt-6.2-astra",
            Self::Sol => "gpt-6.2-sol",
            Self::Luna => "gpt-6-luna",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Opus => "Claude Opus 5.5",
            Self::Sonnet => "Claude Sonnet 5.5",
            Self::Haiku => "Claude Haiku 5.5",
            Self::Astra => "GPT-6.2 Astra",
            Self::Sol => "GPT-6.2 Sol",
            Self::Luna => "GPT-6 Luna",
        }
    }

    pub fn provider(self) -> Provider {
        match self {
            Self::Opus | Self::Sonnet | Self::Haiku => Provider::Anthropic,
            Self::Astra | Self::Sol | Self::Luna => Provider::OpenAi,
        }
    }

    /// The model with the id `id`, if it is on the list.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.id() == id)
    }

    /// The models `provider` offers, in menu order.
    pub fn for_provider(provider: Provider) -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(move |model| model.provider() == provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_models_are_accepted() {
        assert_eq!(Model::from_id("claude-sonnet-5-5"), Some(Model::Sonnet));
        assert_eq!(Model::from_id("claude-opus-5-5"), Some(Model::Opus));
        assert_eq!(Model::from_id("claude-haiku-5-5"), Some(Model::Haiku));
        assert_eq!(Model::from_id("gpt-6.2-sol"), Some(Model::Sol));
        assert_eq!(Model::from_id("claude-fable-5-2"), None);
        assert_eq!(Model::from_id(""), None);
        assert_eq!(Model::default(), Model::Sonnet);
    }

    #[test]
    fn each_provider_offers_its_own_models() {
        for provider in Provider::ALL {
            assert_eq!(Provider::from_id(provider.id()), Some(provider));
            assert_eq!(provider.default_model().provider(), provider);
            let models: Vec<Model> = Model::for_provider(provider).collect();
            assert!(!models.is_empty());
            assert!(models.iter().all(|model| model.provider() == provider));
        }
        assert_eq!(Provider::default(), Provider::Anthropic);
        assert_eq!(Provider::from_id("google"), None);
        assert_eq!(
            Model::for_provider(Provider::Anthropic).collect::<Vec<_>>(),
            [Model::Opus, Model::Sonnet, Model::Haiku]
        );
    }
}
