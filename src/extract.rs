use anyhow::{Context, Result};

/// Extract text from bytes based on MIME type
pub fn extract_text_from_bytes(data: &[u8], mime_type: &str, filename: &str) -> Result<String> {
    match mime_type {
        "application/pdf" => extract_pdf_text(data),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            extract_docx_text(data)
        }
        "text/plain" => Ok(String::from_utf8(data.to_vec())?),
        _ => {
            // Try to infer from filename
            let lower_filename = filename.to_lowercase();
            if lower_filename.ends_with(".pdf") {
                extract_pdf_text(data)
            } else if lower_filename.ends_with(".docx") {
                extract_docx_text(data)
            } else if lower_filename.ends_with(".txt") {
                Ok(String::from_utf8(data.to_vec())?)
            } else {
                Err(anyhow::anyhow!("Unsupported file type: {}", mime_type))
            }
        }
    }
}

/// Extract text from PDF using markdownify
fn extract_pdf_text(data: &[u8]) -> Result<String> {
    use std::io::Write;
    let temp_file = std::env::temp_dir().join(format!("pdf_extract_{}.pdf", std::process::id()));
    let mut file = std::fs::File::create(&temp_file)
        .context("Failed to create temp file")?;
    file.write_all(data)
        .context("Failed to write temp file")?;
    drop(file);

    let markdown = markdownify::pdf::pdf_convert(&temp_file, None)
        .map_err(|e| anyhow::anyhow!("Failed to extract text from PDF: {}", e))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    Ok(markdown)
}

/// Extract text from DOCX using markdownify
fn extract_docx_text(data: &[u8]) -> Result<String> {
    use std::io::Write;
    let temp_file = std::env::temp_dir().join(format!("docx_extract_{}.docx", std::process::id()));
    let mut file = std::fs::File::create(&temp_file)
        .context("Failed to create temp file")?;
    file.write_all(data)
        .context("Failed to write temp file")?;
    drop(file);

    let markdown = markdownify::docx::docx_convert(&temp_file)
        .map_err(|e| anyhow::anyhow!("Failed to extract text from DOCX: {}", e))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    Ok(markdown)
}

/// Check if we can extract text from this document type
pub fn is_extractable_document(mime_type: &str, filename: &str) -> bool {
    match mime_type {
        "application/pdf" => true,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => true,
        "text/plain" => true,
        _ => {
            let lower_filename = filename.to_lowercase();
            lower_filename.ends_with(".pdf")
                || lower_filename.ends_with(".docx")
                || lower_filename.ends_with(".txt")
        }
    }
}

