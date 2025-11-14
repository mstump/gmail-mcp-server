use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::error;

use crate::email::decode_email_content;
use crate::extract::{extract_text_from_bytes, is_extractable_document};
use crate::gmail::{GmailServer, GMAIL_API_BASE};

/// Search Gmail threads
pub async fn search_threads(
    gmail_server: &GmailServer,
    query: &str,
    max_results: i64,
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();
    let url = format!(
        "{}/users/{}/threads?q={}&maxResults={}",
        GMAIL_API_BASE,
        user_id,
        urlencoding::encode(query),
        max_results
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to search threads")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let result: Value = response.json().await.context("Failed to parse response")?;
    Ok(result)
}

/// Create a Gmail draft
pub async fn create_draft(
    gmail_server: &GmailServer,
    to: &str,
    subject: &str,
    body: &str,
    thread_id: Option<&str>,
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    // Build email message in RFC 2822 format
    let mut message = format!("To: {to}\r\n");
    message.push_str(&format!("Subject: {subject}\r\n"));
    message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    message.push_str("\r\n");
    message.push_str(body);

    // Encode message in base64url
    let encoded_message = URL_SAFE.encode(message.as_bytes());

    let mut draft_payload = json!({
        "message": {
            "raw": encoded_message
        }
    });

    // Add thread ID if provided
    if let Some(tid) = thread_id {
        draft_payload["message"]["threadId"] = json!(tid);
    }

    let url = format!("{GMAIL_API_BASE}/users/{user_id}/drafts");

    let response = client
        .post(&url)
        .json(&draft_payload)
        .send()
        .await
        .context("Failed to create draft")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let result: Value = response.json().await.context("Failed to parse response")?;
    Ok(result)
}

/// Extract attachment text by filename
pub async fn extract_attachment_by_filename(
    gmail_server: &GmailServer,
    message_id: &str,
    filename: &str,
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    // Get the message
    let url = format!("{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}");
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to get message")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let message: Value = response.json().await.context("Failed to parse message")?;

    // Find the attachment by filename
    let parts = message["payload"]["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid message structure"))?;

    fn find_attachment(parts: &[Value], filename: &str) -> Option<(String, String)> {
        for part in parts {
            if let Some(part_filename) = part["filename"].as_str() {
                if part_filename == filename {
                    if let Some(att_id) = part["body"]["attachmentId"].as_str() {
                        let mime = part["mimeType"]
                            .as_str()
                            .unwrap_or("application/octet-stream");
                        return Some((att_id.to_string(), mime.to_string()));
                    }
                }
            }
            // Recursively search in nested parts
            if let Some(nested_parts) = part["parts"].as_array() {
                if let Some(result) = find_attachment(nested_parts, filename) {
                    return Some(result);
                }
            }
        }
        None
    }

    let (att_id, mime) = find_attachment(parts, filename)
        .ok_or_else(|| anyhow::anyhow!("Attachment '{filename}' not found in message"))?;

    // Download the attachment
    let att_url =
        format!("{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}/attachments/{att_id}");

    let att_response = client
        .get(&att_url)
        .send()
        .await
        .context("Failed to download attachment")?;

    let att_status = att_response.status();
    if !att_status.is_success() {
        let error_text = att_response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Gmail API error: {att_status} - {error_text}"
        ));
    }

    let att_data: Value = att_response
        .json()
        .await
        .context("Failed to parse attachment")?;
    let encoded_data = att_data["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid attachment data"))?;

    // Decode base64url
    let decoded_data = URL_SAFE
        .decode(encoded_data)
        .context("Failed to decode attachment data")?;

    // Extract text if possible
    if is_extractable_document(&mime, filename) {
        let extracted_text = extract_text_from_bytes(&decoded_data, &mime, filename)
            .context("Failed to extract text from attachment")?;

        Ok(json!({
            "filename": filename,
            "mime_type": mime,
            "size": decoded_data.len(),
            "extracted_text": extracted_text
        }))
    } else {
        Ok(json!({
            "filename": filename,
            "mime_type": mime,
            "size": decoded_data.len(),
            "extracted_text": null,
            "error": "File type not supported for text extraction"
        }))
    }
}

/// Fetch email bodies for threads
pub async fn fetch_email_bodies(
    gmail_server: &GmailServer,
    thread_ids: &[String],
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    let mut results = Vec::new();

    for thread_id in thread_ids {
        let url = format!("{GMAIL_API_BASE}/users/{user_id}/threads/{thread_id}");
        let response = client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to get thread {thread_id}"))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Error fetching thread {}: {}", thread_id, error_text);
            continue;
        }

        let thread: Value = response.json().await.context("Failed to parse thread")?;

        let messages = thread["messages"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid thread structure"))?;

        let mut thread_messages = Vec::new();

        for message in messages {
            let message_id = message["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Message missing ID"))?;

            // Get full message details
            let msg_url = format!("{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}");
            let msg_response = client
                .get(&msg_url)
                .send()
                .await
                .context(format!("Failed to get message {message_id}"))?;

            if !msg_response.status().is_success() {
                continue;
            }

            let msg: Value = msg_response
                .json()
                .await
                .context("Failed to parse message")?;

            // Extract body text
            let body_text = extract_message_body(&msg)?;

            // Extract headers
            let empty_vec = Vec::new();
            let headers = msg["payload"]["headers"].as_array().unwrap_or(&empty_vec);

            let mut from = None;
            let mut subject = None;
            let mut date = None;

            for header in headers {
                let name = header["name"].as_str().unwrap_or("");
                let value = header["value"].as_str().unwrap_or("");
                match name {
                    "From" => from = Some(value.to_string()),
                    "Subject" => subject = Some(value.to_string()),
                    "Date" => date = Some(value.to_string()),
                    _ => {}
                }
            }

            thread_messages.push(json!({
                "message_id": message_id,
                "from": from,
                "subject": subject,
                "date": date,
                "body": body_text
            }));
        }

        results.push(json!({
            "thread_id": thread_id,
            "messages": thread_messages
        }));
    }

    Ok(json!({ "threads": results }))
}

/// Download attachment
pub async fn download_attachment(
    gmail_server: &GmailServer,
    message_id: &str,
    filename: &str,
    download_dir: Option<&str>,
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    // Get the message
    let url = format!("{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}");
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to get message")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let message: Value = response.json().await.context("Failed to parse message")?;

    // Find the attachment by filename
    let parts = message["payload"]["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid message structure"))?;

    fn find_attachment(parts: &[Value], filename: &str) -> Option<(String, String)> {
        for part in parts {
            if let Some(part_filename) = part["filename"].as_str() {
                if part_filename == filename {
                    if let Some(att_id) = part["body"]["attachmentId"].as_str() {
                        let mime = part["mimeType"]
                            .as_str()
                            .unwrap_or("application/octet-stream");
                        return Some((att_id.to_string(), mime.to_string()));
                    }
                }
            }
            if let Some(nested_parts) = part["parts"].as_array() {
                if let Some(result) = find_attachment(nested_parts, filename) {
                    return Some(result);
                }
            }
        }
        None
    }

    let (attachment_id, mime_type) = find_attachment(parts, filename)
        .ok_or_else(|| anyhow::anyhow!("Attachment '{filename}' not found in message"))?;

    // Download the attachment
    let att_url = format!(
        "{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}/attachments/{attachment_id}"
    );

    let att_response = client
        .get(&att_url)
        .send()
        .await
        .context("Failed to download attachment")?;

    let att_status = att_response.status();
    if !att_status.is_success() {
        let error_text = att_response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Gmail API error: {att_status} - {error_text}"
        ));
    }

    let att_data: Value = att_response
        .json()
        .await
        .context("Failed to parse attachment")?;
    let encoded_data = att_data["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid attachment data"))?;

    // Decode base64url
    let decoded_data = URL_SAFE
        .decode(encoded_data)
        .context("Failed to decode attachment data")?;

    // Determine download directory
    let download_path = if let Some(dir) = download_dir {
        PathBuf::from(dir)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // Ensure directory exists
    std::fs::create_dir_all(&download_path).context("Failed to create download directory")?;

    let file_path = download_path.join(filename);

    // Write file
    std::fs::write(&file_path, &decoded_data).context("Failed to write attachment file")?;

    Ok(json!({
        "filename": filename,
        "mime_type": mime_type,
        "size": decoded_data.len(),
        "path": file_path.to_string_lossy().to_string()
    }))
}

/// Forward email
pub async fn forward_email(
    gmail_server: &GmailServer,
    message_id: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    // Get the original message
    let url = format!("{GMAIL_API_BASE}/users/{user_id}/messages/{message_id}");
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to get original message")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let original_message: Value = response.json().await.context("Failed to parse message")?;

    // Extract original message details
    let empty_vec = Vec::new();
    let headers = original_message["payload"]["headers"]
        .as_array()
        .unwrap_or(&empty_vec);

    let mut original_from = None;
    let mut original_subject = None;
    let mut original_date = None;

    for header in headers {
        let name = header["name"].as_str().unwrap_or("");
        let value = header["value"].as_str().unwrap_or("");
        match name {
            "From" => original_from = Some(value.to_string()),
            "Subject" => original_subject = Some(value.to_string()),
            "Date" => original_date = Some(value.to_string()),
            _ => {}
        }
    }

    // Build forwarded message
    let mut message = format!("To: {to}\r\n");
    message.push_str(&format!("Subject: {subject}\r\n"));
    message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    message.push_str("\r\n");
    message.push_str(body);
    message.push_str("\r\n\r\n");
    message.push_str("---------- Forwarded message ----------\r\n");
    if let Some(from) = original_from {
        message.push_str(&format!("From: {from}\r\n"));
    }
    if let Some(date) = original_date {
        message.push_str(&format!("Date: {date}\r\n"));
    }
    if let Some(subj) = original_subject {
        message.push_str(&format!("Subject: {subj}\r\n"));
    }
    message.push_str("\r\n");

    // Get original body
    let original_body = extract_message_body(&original_message)?;
    message.push_str(&original_body);

    // Encode message in base64url
    let encoded_message = URL_SAFE.encode(message.as_bytes());

    // Send the message
    let send_url = format!("{GMAIL_API_BASE}/users/{user_id}/messages/send");
    let send_payload = json!({
        "raw": encoded_message
    });

    let send_response = client
        .post(&send_url)
        .json(&send_payload)
        .send()
        .await
        .context("Failed to send forwarded message")?;

    let send_status = send_response.status();
    if !send_status.is_success() {
        let error_text = send_response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Gmail API error: {send_status} - {error_text}"
        ));
    }

    let result: Value = send_response
        .json()
        .await
        .context("Failed to parse response")?;
    Ok(result)
}

/// Send draft
pub async fn send_draft(gmail_server: &GmailServer, draft_id: &str) -> Result<Value> {
    gmail_server.check_authentication().await?;

    let client = gmail_server.authenticated_client().await?;
    let user_id = gmail_server.user_id();

    let url = format!("{GMAIL_API_BASE}/users/{user_id}/drafts/{draft_id}/send");

    let payload = json!({});

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send draft")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gmail API error: {status} - {error_text}"));
    }

    let result: Value = response.json().await.context("Failed to parse response")?;
    Ok(result)
}

/// Helper function to extract message body from Gmail API response
fn extract_message_body(message: &Value) -> Result<String> {
    let payload = &message["payload"];

    // Check if body is directly in payload
    if let Some(body_data) = payload["body"]["data"].as_str() {
        return decode_email_content(body_data);
    }

    // Check parts for body
    if let Some(parts) = payload["parts"].as_array() {
        // Look for text/plain first, then text/html
        for part in parts {
            let mime_type = part["mimeType"].as_str().unwrap_or("");
            if mime_type == "text/plain" {
                if let Some(body_data) = part["body"]["data"].as_str() {
                    return decode_email_content(body_data);
                }
            }
        }

        // If no plain text, try HTML
        for part in parts {
            let mime_type = part["mimeType"].as_str().unwrap_or("");
            if mime_type == "text/html" {
                if let Some(body_data) = part["body"]["data"].as_str() {
                    return decode_email_content(body_data);
                }
            }
        }

        // Recursively search nested parts
        for part in parts {
            if let Some(nested_parts) = part["parts"].as_array() {
                for nested_part in nested_parts {
                    let mime_type = nested_part["mimeType"].as_str().unwrap_or("");
                    if mime_type == "text/plain" {
                        if let Some(body_data) = nested_part["body"]["data"].as_str() {
                            return decode_email_content(body_data);
                        }
                    }
                }
                for nested_part in nested_parts {
                    let mime_type = nested_part["mimeType"].as_str().unwrap_or("");
                    if mime_type == "text/html" {
                        if let Some(body_data) = nested_part["body"]["data"].as_str() {
                            return decode_email_content(body_data);
                        }
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Could not extract message body"))
}
