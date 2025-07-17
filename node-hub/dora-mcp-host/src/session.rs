use std::sync::{Arc, Mutex};

use eyre::Result;
use serde_json;

use crate::client::ChatClient;
use crate::{
    models::{ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, ToolFunction},
    tool::{Tool as ToolTrait, ToolSet},
};

pub struct ChatSession {
    client: Arc<dyn ChatClient>,
    tool_set: ToolSet,
    model: Option<String>,
    messages: Mutex<Vec<ChatCompletionMessage>>,
}

impl ChatSession {
    pub fn new(client: Arc<dyn ChatClient>, tool_set: ToolSet, model: Option<String>) -> Self {
        Self {
            client,
            tool_set,
            model,
            messages: Default::default(),
        }
    }

    pub fn add_system_prompt(&mut self, prompt: impl ToString) {
        let mut messages = self.messages.lock().expect("messages should locked");
        messages.push(ChatCompletionMessage::system(prompt));
    }

    pub fn get_tools(&self) -> Vec<Arc<dyn ToolTrait>> {
        self.tool_set.tools()
    }

    pub async fn analyze_tool_call(&self, response: &ChatCompletionMessage) {
        let mut tool_calls_func = Vec::new();
        if let Some(tool_calls) = response.tool_calls.as_ref() {
            for tool_call in tool_calls {
                if tool_call.ty == "function" {
                    tool_calls_func.push(tool_call.function.clone());
                }
            }
        } else {
            // check if message contains tool call
            for text in response.content.to_texts() {
                if text.contains("Tool:") {
                    let lines: Vec<&str> = text.split('\n').collect();
                    // simple parse tool call
                    let mut tool_name = None;
                    let mut args_text = Vec::new();
                    let mut parsing_args = false;

                    for line in lines {
                        if line.starts_with("Tool:") {
                            tool_name = line.strip_prefix("Tool:").map(|s| s.trim().to_string());
                            parsing_args = false;
                        } else if line.starts_with("Inputs:") {
                            parsing_args = true;
                        } else if parsing_args {
                            args_text.push(line.trim());
                        }
                    }
                    if let Some(name) = tool_name {
                        tool_calls_func.push(ToolFunction {
                            name,
                            arguments: args_text.join("\n"),
                        });
                    }
                }
            }
        }
        // call tool
        for tool_call in tool_calls_func {
            println!("tool call: {:?}", tool_call);
            let tool = self.tool_set.get_tool(&tool_call.name);
            if let Some(tool) = tool {
                // call tool
                let args = serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                    .unwrap_or_default();
                match tool.call(args).await {
                    Ok(result) => {
                        if result.is_error.is_some_and(|b| b) {
                            let mut messages =
                                self.messages.lock().expect("messages should locked");
                            messages.push(ChatCompletionMessage::user(
                                "tool call failed, mcp call error",
                            ));
                        } else {
                            result.content.iter().for_each(|content| {
                                if let Some(content_text) = content.as_text() {
                                    let json_result = serde_json::from_str::<serde_json::Value>(
                                        &content_text.text,
                                    )
                                    .unwrap_or_default();
                                    let pretty_result =
                                        serde_json::to_string_pretty(&json_result).unwrap();
                                    println!("call tool result: {}", pretty_result);
                                    let mut messages =
                                        self.messages.lock().expect("messages should locked");
                                    messages.push(ChatCompletionMessage::user(format!(
                                        "call tool result: {}",
                                        pretty_result
                                    )));
                                }
                            });
                        }
                    }
                    Err(e) => {
                        println!("tool call failed: {}", e);
                        let mut messages = self.messages.lock().expect("messages should locked");
                        messages.push(ChatCompletionMessage::user(format!(
                            "tool call failed: {}",
                            e
                        )));
                    }
                }
            } else {
                println!("tool not found: {}", tool_call.name);
            }
        }
    }
    pub async fn chat(&self, mut request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        {
            let mut messages = self.messages.lock().expect("messages should locked");
            for message in request.messages.iter() {
                messages.push(message.clone());
            }
        }
        let tools = self.tool_set.tools();
        let tool_definitions = if !tools.is_empty() {
            Some(
                tools
                    .iter()
                    .map(|tool| crate::models::ToolInfo {
                        name: tool.name(),
                        description: tool.description(),
                        parameters: tool.parameters(),
                    })
                    .collect(),
            )
        } else {
            None
        };

        request.model = self.model.clone();
        request.tools = tool_definitions;

        // send request
        let response = self.client.complete(request).await?;
        // get choice
        if let Some(choice) = response.choices.first() {
            println!("AI > {:#?}", choice.message.to_texts());
            // analyze tool call
            self.analyze_tool_call(&choice.message).await;
        }

        Ok(response)
    }
}
