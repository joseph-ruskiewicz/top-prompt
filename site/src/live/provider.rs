//! The model providers a visitor can choose, the models each one offers, and the
//! provider-neutral interface to their APIs. Each `Provider` method dispatches to
//! the provider's adapter module, so the conversation code never sees a wire format.

use serde_json::Value;

use super::{
    anthropic,
    api::{ApiError, Update},
    key::ApiKey,
    openai, sse,
};

/// Whose API answers the visitor. One key for one provider at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Provider {
    #[default]
    Anthropic,
    OpenAi,
}

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

    /// Where the visitor manages this provider's keys and billing, for a sentence.
    pub fn console(self) -> &'static str {
        match self {
            Self::Anthropic => "the Claude Console",
            Self::OpenAi => "the OpenAI dashboard",
        }
    }

    /// How to make a key that cannot cost much, for a sentence after "create one".
    pub fn key_advice(self) -> &'static str {
        match self {
            Self::Anthropic => "in a workspace with a low spend limit",
            Self::OpenAi => "with restricted permissions, in a project with a low budget",
        }
    }

    /// What to do about a billing error.
    pub fn billing_hint(self) -> &'static str {
        match self {
            Self::Anthropic => {
                "Add credit or raise the spend limit under Billing in the Claude Console."
            }
            Self::OpenAi => "Add credit under Billing in the OpenAI dashboard.",
        }
    }

    /// What to do when the key may not use a model.
    pub fn permission_hint(self) -> &'static str {
        match self {
            Self::Anthropic => "Check the key's workspace in the Claude Console.",
            Self::OpenAi => {
                "Check the key's permissions and the project's allowed models in the OpenAI \
                 dashboard."
            }
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

impl Provider {
    /// The history items for a user message with `text`.
    pub fn user_items(self, text: &str) -> Vec<Value> {
        match self {
            Self::Anthropic => anthropic::user_items(text),
            Self::OpenAi => openai::user_items(text),
        }
    }

    /// The history items for the text of a reply that was stopped or failed, kept
    /// as the assistant's. `Visitor::stop` calls it with the visitor's lock held, so
    /// no adapter may panic here.
    pub fn interrupted_items(self, text: &str) -> Vec<Value> {
        match self {
            Self::Anthropic => anthropic::interrupted_items(text),
            Self::OpenAi => openai::interrupted_items(text),
        }
    }

    /// The history items for a round's tool results: each call, its output text and
    /// whether it is an error.
    pub fn tool_result_items(self, results: &[(ToolCall, String, bool)]) -> Vec<Value> {
        match self {
            Self::Anthropic => anthropic::tool_result_items(results),
            Self::OpenAi => openai::tool_result_items(results),
        }
    }

    /// The streaming request body for `model` on `history`.
    pub fn request_body(self, model: Model, history: &[Value]) -> Value {
        match self {
            Self::Anthropic => anthropic::request_body(model, history),
            Self::OpenAi => openai::request_body(model, history),
        }
    }

    /// The history items, in order, made ready to send.
    pub fn prepare_history(self, items: Vec<Value>) -> Vec<Value> {
        match self {
            Self::Anthropic => anthropic::prepare_history(items),
            // The Responses API takes the items as they are.
            Self::OpenAi => items,
        }
    }

    /// The state for reading one streamed response.
    pub fn stream_state(self) -> StreamState {
        match self {
            Self::Anthropic => StreamState::Anthropic(anthropic::Accumulator::default()),
            Self::OpenAi => StreamState::OpenAi(openai::Accumulator::default()),
        }
    }

    /// The error for an error response with `status` and `body`, or with no
    /// status, for an error event's data in a stream. Any copy of `key` is removed.
    pub fn parse_error(
        self,
        status: Option<u16>,
        body: &str,
        retry_after: Option<u64>,
        key: &ApiKey,
    ) -> ApiError {
        match self {
            Self::Anthropic => anthropic::parse_error(status, body, retry_after, key),
            Self::OpenAi => openai::parse_error(status, body, retry_after, key),
        }
    }
}

/// How a response ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The model called tools and waits for their results.
    ToolUse,
    /// The response reached [`super::api::MAX_TOKENS`].
    MaxTokens,
    /// The model declined to continue.
    Refusal,
    /// The model finished its answer.
    Done,
}

/// A tool call the model made.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// The input, or `Err` with the raw text when it was not valid JSON.
    pub input: Result<Value, String>,
}

/// A response that streamed to its end.
#[derive(Debug)]
pub struct Finished {
    pub outcome: Outcome,
    pub calls: Vec<ToolCall>,
    /// The history items for the model's output, to append exactly as given.
    pub items: Vec<Value>,
}

/// Reads one streamed response, in the provider's format.
pub enum StreamState {
    Anthropic(anthropic::Accumulator),
    OpenAi(openai::Accumulator),
}

impl StreamState {
    /// Applies one event. An error event becomes an [`ApiError`].
    pub fn apply(&mut self, event: &sse::Event, key: &ApiKey) -> Result<Vec<Update>, ApiError> {
        match self {
            Self::Anthropic(message) => Ok(message.apply(event, key)?.into_iter().collect()),
            Self::OpenAi(response) => response.apply(event, key),
        }
    }

    /// The finished response, once the provider's terminal event has arrived.
    pub fn finished(&self) -> Option<Finished> {
        match self {
            Self::Anthropic(message) => message.finished(),
            Self::OpenAi(response) => response.finished(),
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
