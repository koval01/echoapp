use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;
use reqwest::Client;
use serde_json::Value;

#[derive(Clone)]
pub struct TelegramLayer {
    client: Client,
    bot_token: String,
    chat_id: String,
    buffer: Arc<Mutex<Vec<String>>>,
    last_flush: Arc<Mutex<Instant>>,
    max_batch_size: usize,
    max_message_length: usize,
}

impl TelegramLayer {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
            chat_id,
            buffer: Arc::new(Mutex::new(Vec::new())),
            last_flush: Arc::new(Mutex::new(Instant::now())),
            max_batch_size: 5,
            max_message_length: 4000,
        }
    }

    async fn send_to_telegram(&self, message: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": message,
                "parse_mode": "HTML"
            }))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            eprintln!("Failed to send message to Telegram: {} - {}", status, error_text);
        }

        Ok(())
    }

    async fn flush_buffer(&self) {
        let mut buffer = self.buffer.lock().await;
        if buffer.is_empty() {
            return;
        }

        let mut current_batch = Vec::new();
        let mut current_length = 0;

        for log in buffer.drain(..) {
            let log_length = log.len() + 2;

            if current_length + log_length > self.max_message_length && !current_batch.is_empty() {
                if let Err(e) = self.send_batch(&current_batch).await {
                    eprintln!("Failed to send logs to Telegram: {}", e);
                }
                current_batch.clear();
                current_length = 0;
            }

            if log_length > self.max_message_length {
                if !current_batch.is_empty() {
                    if let Err(e) = self.send_batch(&current_batch).await {
                        eprintln!("Failed to send logs to Telegram: {}", e);
                    }
                    current_batch.clear();
                    current_length = 0;
                }

                let chunks = Self::split_long_message(&log, self.max_message_length);
                for chunk in chunks {
                    if let Err(e) = self.send_to_telegram(chunk).await {
                        eprintln!("Failed to send log chunk to Telegram: {}", e);
                    }
                }
            } else {
                current_batch.push(log);
                current_length += log_length;
            }

            if current_batch.len() >= self.max_batch_size {
                if let Err(e) = self.send_batch(&current_batch).await {
                    eprintln!("Failed to send logs to Telegram: {}", e);
                }
                current_batch.clear();
                current_length = 0;
            }
        }

        if !current_batch.is_empty() {
            if let Err(e) = self.send_batch(&current_batch).await {
                eprintln!("Failed to send logs to Telegram: {}", e);
            }
        }

        *self.last_flush.lock().await = Instant::now();
    }

    async fn send_batch(&self, batch: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = batch.join("\n-----\n");
        self.send_to_telegram(message).await
    }

    fn split_long_message(message: &str, max_length: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for line in message.lines() {
            if current_chunk.len() + line.len() + 1 > max_length {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                    current_chunk = String::new();
                }

                if line.len() > max_length {
                    let line_chunks: Vec<String> = line
                        .chars()
                        .collect::<Vec<_>>()
                        .chunks(max_length - 100)
                        .map(|chunk| format!("[...] {}", chunk.iter().collect::<String>()))
                        .collect();
                    chunks.extend(line_chunks);
                } else {
                    current_chunk.push_str(line);
                }
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push('\n');
                }
                current_chunk.push_str(line);
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    fn extract_fields_from_json(json_str: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();

        if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
            if let Some(obj) = parsed.as_object() {
                for (key, value) in obj {
                    match value {
                        Value::String(s) => {
                            fields.insert(key.clone(), s.clone());
                        }
                        _ => {
                            fields.insert(key.clone(), value.to_string());
                        }
                    }
                }
            }
        } else {
            Self::parse_fields_manually(json_str, &mut fields);
        }

        fields
    }

    fn parse_fields_manually(json_str: &str, fields: &mut HashMap<String, String>) {
        let content = json_str.trim_start_matches('{').trim_end_matches('}');
        let mut in_quotes = false;
        let mut escape_next = false;
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut parsing_key = true;

        for ch in content.chars() {
            match ch {
                '"' if !escape_next => {
                    in_quotes = !in_quotes;
                    if !in_quotes && parsing_key {
                        parsing_key = false;
                    }
                }
                ':' if !in_quotes => {
                    parsing_key = false;
                }
                ',' if !in_quotes => {
                    if !current_key.is_empty() {
                        fields.insert(current_key.trim().to_string(), current_value.trim().to_string());
                    }
                    current_key.clear();
                    current_value.clear();
                    parsing_key = true;
                }
                _ => {
                    if parsing_key {
                        current_key.push(ch);
                    } else {
                        current_value.push(ch);
                    }
                    escape_next = ch == '\\';
                }
            }
        }

        if !current_key.is_empty() {
            fields.insert(current_key.trim().to_string(), current_value.trim().to_string());
        }

        for value in fields.values_mut() {
            *value = value.trim_matches('"').to_string();
        }
    }

    fn format_log_entry(metadata: &tracing::Metadata, fields: &str) -> Option<String> {
        let extracted_fields = Self::extract_fields_from_json(fields);

        // Отправляем только ERROR логи с определенными целевыми именами
        if metadata.level() != &tracing::Level::ERROR {
            return None;
        }

        let target = metadata.target();
        if !target.contains("echoapp") {
            return None;
        }

        // Создаем owned строки для всех значений
        let unknown = "Unknown".to_string();

        let instance = extracted_fields.get("instance").unwrap_or(&unknown).clone();
        let request_id = extracted_fields.get("request_id").unwrap_or(&unknown).clone();
        let path = extracted_fields.get("path").unwrap_or(&unknown).clone();
        let method = extracted_fields.get("method").unwrap_or(&unknown).clone();
        let status = extracted_fields.get("status").unwrap_or(&unknown).clone();
        let error_type = extracted_fields.get("error_type").unwrap_or(&unknown).clone();
        let message = extracted_fields.get("message").unwrap_or(&unknown).clone();
        let module = extracted_fields.get("module").unwrap_or(&unknown).clone();
        let file = extracted_fields.get("file").unwrap_or(&unknown).clone();
        let line = extracted_fields.get("line")
            .or(extracted_fields.get("line_number"))
            .unwrap_or(&unknown)
            .clone();

        // Используем текущее время с time crate
        let current_time = if let Ok(now) = time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339) {
            now
        } else {
            "Time format error".to_string()
        };

        Some(format!(
            "📅 <b>Time:</b> <code>{}</code>\n\
         🔧 <b>Instance:</b> <code>{}</code>\n\
         🌐 <b>Request ID:</b> <code>{}</code>\n\
         📍 <b>Path:</b> <code>{}</code>\n\
         ⚡ <b>Method:</b> <code>{}</code>\n\
         📊 <b>Status:</b> <code>{}</code>\n\
         🎯 <b>Error Type:</b> <code>{}</code>\n\
         📝 <b>Message:</b> <code>{}</code>\n\
         📁 <b>Module:</b> <code>{}</code>\n\
         📄 <b>File:</b> <code>{}:{}</code>",
            current_time,
            instance,
            request_id,
            path,
            method,
            status,
            error_type,
            message,
            module,
            file,
            line,
        ))
    }

    async fn add_log(&self, log_entry: String) {
        let mut buffer = self.buffer.lock().await;
        buffer.push(log_entry);

        if buffer.len() >= self.max_batch_size {
            drop(buffer);
            self.flush_buffer().await;
        }
    }

    pub fn start_background_flush(self, flush_interval: Duration) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            loop {
                interval.tick().await;
                self.flush_buffer().await;
            }
        });
    }
}

struct JsonVisitor {
    fields: Vec<(String, String)>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            fields: Vec::new(),
        }
    }

    fn to_json_string(&self) -> String {
        let mut json_parts = Vec::new();
        for (key, value) in &self.fields {
            let escaped_value = value.replace('"', "\\\"");
            json_parts.push(format!("\"{}\":\"{}\"", key, escaped_value));
        }
        format!("{{{}}}", json_parts.join(","))
    }
}

impl tracing::field::Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        self.fields.push((field.name().to_string(), value_str));
    }
}

impl<S> Layer<S> for TelegramLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);

        let json_fields = visitor.to_json_string();

        if let Some(log_entry) = Self::format_log_entry(metadata, &json_fields) {
            let layer = self.clone();
            tokio::spawn(async move {
                layer.add_log(log_entry).await;
            });
        }
    }
}

pub fn init_telegram_logging(bot_token: String, chat_id: String) -> Option<TelegramLayer> {
    if bot_token.is_empty() || chat_id.is_empty() {
        return None;
    }

    let layer = TelegramLayer::new(bot_token, chat_id);
    layer.clone().start_background_flush(Duration::from_secs(10));

    Some(layer)
}
