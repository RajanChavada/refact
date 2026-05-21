use std::collections::HashSet;
use crate::call_validation::{ChatContent, ChatMessage};

pub fn apply_summarization_linearize(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    if !messages.iter().any(|m| m.role == "summarization") {
        return messages;
    }

    let summaries: Vec<(usize, usize, String)> = messages
        .iter()
        .filter(|m| m.role == "summarization")
        .filter_map(|m| {
            let (start, end) = m.summarized_range?;
            Some((start, end, m.content.content_text_only()))
        })
        .collect();

    let mut suppressed: HashSet<usize> = HashSet::new();
    for (start, end, _) in &summaries {
        for i in *start..=*end {
            suppressed.insert(i);
        }
    }

    let mut result = Vec::with_capacity(messages.len());
    let mut emitted_summaries: HashSet<usize> = HashSet::new();

    for (i, msg) in messages.iter().enumerate() {
        if msg.role == "summarization" {
            continue;
        }
        if suppressed.contains(&i) {
            for (start, _, content) in &summaries {
                if i == *start && !emitted_summaries.contains(start) {
                    result.push(ChatMessage {
                        role: "cd_instruction".to_string(),
                        content: ChatContent::SimpleText(content.clone()),
                        ..Default::default()
                    });
                    emitted_summaries.insert(*start);
                }
            }
            continue;
        }
        result.push(msg.clone());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::SimpleText(text.to_string()),
            ..Default::default()
        }
    }

    fn assistant(text: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: ChatContent::SimpleText(text.to_string()),
            ..Default::default()
        }
    }

    fn summarization(content: &str, range: (usize, usize)) -> ChatMessage {
        ChatMessage {
            role: "summarization".to_string(),
            content: ChatContent::SimpleText(content.to_string()),
            summarized_range: Some(range),
            summarization_tier: Some("tier0_deterministic".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_linearize_no_summarization_unchanged() {
        let messages = vec![user("hello"), assistant("hi"), user("world")];
        let result = apply_summarization_linearize(messages.clone());
        assert_eq!(result.len(), messages.len());
        assert_eq!(result[0].content.content_text_only(), "hello");
        assert_eq!(result[1].content.content_text_only(), "hi");
        assert_eq!(result[2].content.content_text_only(), "world");
    }

    #[test]
    fn test_linearize_summarization_replaces_range() {
        let messages = vec![
            user("hello"),          // 0
            assistant("response1"), // 1
            user("follow up"),      // 2
            assistant("response2"), // 3
            user("new question"),   // 4
            summarization("Summary of messages 1-3", (1, 3)),
            assistant("final"),     // 6
        ];
        let result = apply_summarization_linearize(messages);
        let roles: Vec<&str> = result.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["user", "cd_instruction", "user", "assistant"]);
        assert_eq!(result[0].content.content_text_only(), "hello");
        assert_eq!(result[1].content.content_text_only(), "Summary of messages 1-3");
        assert_eq!(result[2].content.content_text_only(), "new question");
        assert_eq!(result[3].content.content_text_only(), "final");
    }

    #[test]
    fn test_linearize_summarization_without_range_is_dropped() {
        let msg_no_range = ChatMessage {
            role: "summarization".to_string(),
            content: ChatContent::SimpleText("orphan summary".to_string()),
            summarized_range: None,
            ..Default::default()
        };
        let messages = vec![user("hello"), msg_no_range, assistant("hi")];
        let result = apply_summarization_linearize(messages);
        let roles: Vec<&str> = result.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["user", "assistant"]);
    }

    #[test]
    fn test_linearize_messages_after_summarized_range_preserved() {
        let messages = vec![
            user("msg0"),             // 0
            assistant("msg1"),        // 1
            user("msg2"),             // 2 - in range
            summarization("sum", (2, 2)),
            user("msg3"),             // 4
        ];
        let result = apply_summarization_linearize(messages);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].content.content_text_only(), "msg0");
        assert_eq!(result[1].content.content_text_only(), "msg1");
        assert_eq!(result[2].content.content_text_only(), "sum");
        assert_eq!(result[3].content.content_text_only(), "msg3");
    }
}
