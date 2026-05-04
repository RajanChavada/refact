#[cfg(test)]
mod background_tasks;
#[cfg(test)]
mod buddy;
#[cfg(test)]
mod caps;
#[cfg(test)]
mod custom_error;
#[cfg(test)]
mod global_context;
#[cfg(test)]
mod indexing_utils;
#[cfg(test)]
mod json_utils;
#[cfg(test)]
mod nicer_logs;
#[cfg(test)]
mod version;
#[cfg(test)]
mod yaml_configs;

#[cfg(test)]
mod ast;
#[cfg(test)]
mod at_commands;
#[cfg(test)]
mod completion_cache;
#[cfg(test)]
mod file_filter;
#[cfg(test)]
mod files_blocklist;
#[cfg(test)]
mod files_correction;
#[cfg(test)]
mod files_in_jsonl;
#[cfg(test)]
mod files_in_workspace;
#[cfg(test)]
mod fuzzy_search;
#[cfg(test)]
mod postprocessing;
#[cfg(test)]
mod scratchpad_abstract;
#[cfg(test)]
mod scratchpads;
#[cfg(test)]
mod subchat;
#[cfg(test)]
mod tokens;
#[cfg(test)]
mod tools;
#[cfg(test)]
mod vecdb;

#[cfg(test)]
mod fetch_embedding;
#[cfg(test)]
mod forward_to_openai_endpoint;
#[cfg(test)]
mod llm;
#[cfg(test)]
mod providers;
#[cfg(test)]
mod restream;
pub mod worktrees;

#[cfg(test)]
mod call_validation;
#[cfg(test)]
mod chat;
#[cfg(test)]
mod http;
#[cfg(test)]
mod lsp;

#[cfg(test)]
mod agentic;
#[cfg(test)]
pub mod constants;
#[cfg(test)]
mod ext;
#[cfg(test)]
mod files_correction_cache;
#[cfg(test)]
mod git;
#[cfg(test)]
mod integrations;
#[cfg(test)]
mod knowledge_graph;
#[cfg(test)]
mod knowledge_index;
#[cfg(test)]
mod memories;
#[cfg(test)]
mod privacy;
#[cfg(test)]
mod stats;
#[cfg(test)]
mod tasks;
#[cfg(test)]
mod trajectory_memos;
#[cfg(test)]
mod voice;
