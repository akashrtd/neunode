use serde::{Deserialize, Serialize};

use crate::error::{InferenceError, Result};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub stream: Option<bool>,
    pub stop: Option<Vec<String>>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
}

impl ChatCompletionRequest {
    pub fn validate(&self) -> Result<()> {
        if self.messages.is_empty() {
            return Err(InferenceError::InvalidRequest("messages is required".to_string()));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(InferenceError::InvalidRequest(
                    "temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }

        if let Some(max_tok) = self.max_tokens {
            if max_tok == 0 {
                return Err(InferenceError::InvalidRequest(
                    "max_tokens must be greater than 0".to_string(),
                ));
            }
        }

        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(InferenceError::InvalidRequest(
                    "top_p must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        if let Some(ref stop) = self.stop {
            if stop.len() > 4 {
                return Err(InferenceError::InvalidRequest(
                    "stop must have at most 4 entries".to_string(),
                ));
            }
        }

        if let Some(fp) = self.frequency_penalty {
            if !(-2.0..=2.0).contains(&fp) {
                return Err(InferenceError::InvalidRequest(
                    "frequency_penalty must be between -2.0 and 2.0".to_string(),
                ));
            }
        }

        if let Some(pp) = self.presence_penalty {
            if !(-2.0..=2.0).contains(&pp) {
                return Err(InferenceError::InvalidRequest(
                    "presence_penalty must be between -2.0 and 2.0".to_string(),
                ));
            }
        }

        let has_user = self.messages.iter().any(|m| m.role == MessageRole::User);
        if !has_user {
            return Err(InferenceError::InvalidRequest(
                "at least one message must have role user".to_string(),
            ));
        }

        Ok(())
    }

    pub fn estimate_tokens(&self) -> u32 {
        let total_chars: usize = self.messages.iter().map(|m| m.content.len()).sum();
        (total_chars as u32) / 4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: neunode_core::types::Timestamp,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: neunode_core::types::Timestamp,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChatMessage,
    pub finish_reason: Option<FinishReason>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage { role, content: content.to_string(), name: None }
    }

    fn valid_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "neunode/llama-3b".to_string(),
            messages: vec![msg(MessageRole::User, "Hello, world!")],
            temperature: Some(0.7),
            max_tokens: Some(256),
            top_p: None,
            stream: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    #[test]
    fn message_role_serde_roundtrip() {
        for role in
            [MessageRole::System, MessageRole::User, MessageRole::Assistant, MessageRole::Tool]
        {
            let json = serde_json::to_string(&role).unwrap();
            let back: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, back);
        }
    }

    #[test]
    fn message_role_lowercase_json() {
        let json = serde_json::to_string(&FinishReason::ToolCalls).unwrap();
        assert!(json.contains("tool_calls"));
    }

    #[test]
    fn message_role_display() {
        assert_eq!(MessageRole::System.to_string(), "system");
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::Tool.to_string(), "tool");
    }

    #[test]
    fn chat_message_serde_roundtrip() {
        let m = ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
            name: Some("agent1".to_string()),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn chat_message_skips_none_name() {
        let m = msg(MessageRole::User, "hi");
        let val: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(!val.as_object().unwrap().contains_key("name"));
    }

    #[test]
    fn request_serde_roundtrip() {
        let req = valid_request();
        let json = serde_json::to_string(&req).unwrap();
        let back: ChatCompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn finish_reason_serde_roundtrip() {
        for reason in [
            FinishReason::Stop,
            FinishReason::Length,
            FinishReason::ContentFilter,
            FinishReason::ToolCalls,
        ] {
            let json = serde_json::to_string(&reason).unwrap();
            let back: FinishReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, back);
        }
    }

    #[test]
    fn finish_reason_snake_case() {
        assert!(serde_json::to_string(&FinishReason::ContentFilter)
            .unwrap()
            .contains("content_filter"));
        assert!(serde_json::to_string(&FinishReason::ToolCalls).unwrap().contains("tool_calls"));
    }

    #[test]
    fn usage_serde_roundtrip() {
        let u = Usage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 };
        let json = serde_json::to_string(&u).unwrap();
        let back: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(u, back);
    }

    #[test]
    fn choice_serde_roundtrip() {
        let c = Choice {
            index: 0,
            message: msg(MessageRole::Assistant, "Hi"),
            finish_reason: FinishReason::Stop,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Choice = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn response_serde_roundtrip() {
        let r = ChatCompletionResponse {
            id: "chatcmpl-abc".to_string(),
            object: "chat.completion".to_string(),
            created: 1700000000,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: msg(MessageRole::Assistant, "Hello!"),
                finish_reason: FinishReason::Stop,
            }],
            usage: Usage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: ChatCompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn chunk_serde_roundtrip() {
        let c = ChatCompletionChunk {
            id: "chatcmpl-xyz".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1700000001,
            model: "llama-3b".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: MessageRole::Assistant,
                    content: "world".to_string(),
                    name: None,
                },
                finish_reason: None,
            }],
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: ChatCompletionChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn validate_valid_request() {
        assert!(valid_request().validate().is_ok());
    }

    #[test]
    fn validate_empty_messages() {
        let req = ChatCompletionRequest { messages: vec![], ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("messages")));
    }

    #[test]
    fn validate_no_user_message() {
        let req = ChatCompletionRequest {
            messages: vec![
                msg(MessageRole::System, "You are helpful"),
                msg(MessageRole::Assistant, "Hi there"),
            ],
            ..valid_request()
        };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("user")));
    }

    #[test]
    fn validate_temperature_too_high() {
        let req = ChatCompletionRequest { temperature: Some(3.0), ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("temperature")));
    }

    #[test]
    fn validate_temperature_negative() {
        let req = ChatCompletionRequest { temperature: Some(-0.1), ..valid_request() };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_temperature_boundary_ok() {
        let r1 = ChatCompletionRequest { temperature: Some(0.0), ..valid_request() };
        let r2 = ChatCompletionRequest { temperature: Some(2.0), ..valid_request() };
        assert!(r1.validate().is_ok());
        assert!(r2.validate().is_ok());
    }

    #[test]
    fn validate_max_tokens_zero() {
        let req = ChatCompletionRequest { max_tokens: Some(0), ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("max_tokens")));
    }

    #[test]
    fn validate_top_p_out_of_range() {
        let req = ChatCompletionRequest { top_p: Some(1.5), ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("top_p")));
    }

    #[test]
    fn validate_stop_too_many() {
        let req = ChatCompletionRequest {
            stop: Some(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
            ]),
            ..valid_request()
        };
        let err = req.validate().unwrap_err();
        assert!(matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("stop")));
    }

    #[test]
    fn validate_stop_exactly_four_ok() {
        let req = ChatCompletionRequest {
            stop: Some(vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()]),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn validate_frequency_penalty_out_of_range() {
        let req = ChatCompletionRequest { frequency_penalty: Some(3.0), ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(
            matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("frequency_penalty"))
        );
    }

    #[test]
    fn validate_presence_penalty_out_of_range() {
        let req = ChatCompletionRequest { presence_penalty: Some(-3.0), ..valid_request() };
        let err = req.validate().unwrap_err();
        assert!(
            matches!(err, InferenceError::InvalidRequest(ref s) if s.contains("presence_penalty"))
        );
    }

    #[test]
    fn validate_penalties_boundary_ok() {
        let req = ChatCompletionRequest {
            frequency_penalty: Some(-2.0),
            presence_penalty: Some(2.0),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn estimate_tokens_basic() {
        let req = valid_request();
        assert_eq!(req.estimate_tokens(), 3);
    }

    #[test]
    fn estimate_tokens_empty_content() {
        let req = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "".to_string(),
                name: None,
            }],
            ..valid_request()
        };
        assert_eq!(req.estimate_tokens(), 0);
    }

    #[test]
    fn estimate_tokens_multi_message() {
        let req = ChatCompletionRequest {
            messages: vec![
                msg(MessageRole::System, "You are helpful"),
                msg(MessageRole::User, "What is 2+2?"),
                msg(MessageRole::Assistant, "4"),
                msg(MessageRole::User, "Thanks!"),
            ],
            ..valid_request()
        };
        assert_eq!(req.estimate_tokens(), 8);
    }

    #[test]
    fn request_all_optional_none_validates() {
        let req = ChatCompletionRequest {
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn response_json_structure() {
        let r = ChatCompletionResponse {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion".to_string(),
            created: 1700000000,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: msg(MessageRole::Assistant, "Hi"),
                finish_reason: FinishReason::Stop,
            }],
            usage: Usage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        };
        let val: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert_eq!(val["object"], "chat.completion");
        assert_eq!(val["model"], "gpt-4");
        assert!(val["choices"].is_array());
        assert_eq!(val["usage"]["total_tokens"], 30);
    }
}
