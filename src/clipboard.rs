/// Copy `text` to the system clipboard.
/// Tries wl-copy (Wayland), then xclip, then xsel.
pub fn copy_to_clipboard(text: &str) {
    let tools: &[(&str, &[&str])] = &[
        ("wl-copy",  &[]),
        ("xclip",    &["-selection", "clipboard"]),
        ("xsel",     &["--clipboard", "--input"]),
    ];
    for (cmd, args) in tools {
        if let Ok(mut child) = std::process::Command::new(cmd)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
}

pub struct CodeBlock {
    pub start_fence: usize,  // line index of opening ```
    pub end_fence:   usize,  // line index of closing ``` (or last line if unclosed)
    pub content:     String, // lines between the fences, joined with '\n'
}

/// Parse all fenced code blocks (``` ... ```) from `lines`.
pub fn extract_code_blocks(lines: &[String]) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut open: Option<usize> = None;
    let mut buf: Vec<&str> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            match open {
                None => {
                    open = Some(i);
                    buf.clear();
                }
                Some(start) => {
                    blocks.push(CodeBlock {
                        start_fence: start,
                        end_fence:   i,
                        content:     buf.join("\n"),
                    });
                    open = None;
                    buf.clear();
                }
            }
        } else if open.is_some() {
            buf.push(line.as_str());
        }
    }
    // Unclosed fence
    if let Some(start) = open {
        blocks.push(CodeBlock {
            start_fence: start,
            end_fence:   lines.len().saturating_sub(1),
            content:     buf.join("\n"),
        });
    }
    blocks
}

/// Find the code block whose fence range contains `line_index`.
pub fn block_at_line(blocks: &[CodeBlock], line_index: usize) -> Option<&CodeBlock> {
    blocks.iter().find(|b| line_index >= b.start_fence && line_index <= b.end_fence)
}

