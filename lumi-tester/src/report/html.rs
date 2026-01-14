use super::types::TestResults;
use crate::runner::state::{CommandStatus, FlowStatus};
use anyhow::Result;
use std::path::Path;

/// Generate HTML report
pub async fn generate(results: &TestResults, output: Option<&Path>) -> Result<()> {
    let html = generate_html(results);

    if let Some(path) = output {
        std::fs::write(path, html)?;
        println!("HTML report saved to: {}", path.display());
    } else {
        println!("{}", html);
    }

    Ok(())
}

fn generate_html(results: &TestResults) -> String {
    let summary = &results.summary;
    let pass_rate = if summary.total_commands > 0 {
        (summary.passed as f64 / summary.total_commands as f64 * 100.0) as u32
    } else {
        0
    };

    let mut flows_html = String::new();
    for flow in &results.flows {
        let (flow_status_text, flow_status_class) = match flow.status {
            FlowStatus::Passed => ("Passed", "passed"),
            FlowStatus::Failed => ("Failed", "failed"),
            _ => ("Partial", "partial"),
        };

        let mut commands_html = String::new();
        for cmd in &flow.commands {
            let (status_icon, status_class) = match &cmd.status {
                CommandStatus::Passed => ("✓", "passed"),
                CommandStatus::Failed { .. } => ("✗", "failed"),
                CommandStatus::Skipped { .. } => ("○", "skipped"),
                CommandStatus::Running => ("⋯", "running"),
                CommandStatus::Pending => ("○", "pending"),
                CommandStatus::Retrying { .. } => ("↻", "retrying"),
            };

            let screenshot_html = if let Some(path) = &cmd.screenshot_path {
                format!(
                    r##"<a href="#" class="screenshot-link" onclick="showScreenshot('{}')">📸 View Screenshot</a>"##,
                    path
                )
            } else {
                String::new()
            };

            let error_html = match &cmd.status {
                CommandStatus::Failed { error } => {
                    format!(
                        r##"<div class="error-message">{}</div>"##,
                        html_escape(error)
                    )
                }
                _ => String::new(),
            };

            let duration_html = cmd
                .duration_ms
                .map(|d| format!("<span class=\"duration\">{}ms</span>", d))
                .unwrap_or_default();

            let onclick = if let Some(path) = &cmd.screenshot_path {
                format!("showScreenshot('{}')", path)
            } else {
                "".to_string()
            };

            commands_html.push_str(&format!(
                r##"
                <div class="command {status_class}" onclick="{onclick}">
                    <div class="command-icon">{status_icon}</div>
                    <div class="command-content">
                        <div class="command-name">{}</div>
                        <div class="command-meta">
                            {duration_html}
                            {screenshot_html}
                        </div>
                        {error_html}
                    </div>
                </div>
            "##,
                html_escape(&cmd.command_display),
                status_class = status_class,
                status_icon = status_icon,
                duration_html = duration_html,
                screenshot_html = screenshot_html,
                error_html = error_html,
                onclick = onclick
            ));
        }

        let duration_html = flow
            .total_duration_ms
            .map(|d| format!("<span class=\"duration\">{}ms</span>", d))
            .unwrap_or_default();

        let video_html = if let Some(path) = &flow.video_path {
            format!(
                r#"
                <div class="video-details">
                    <details>
                        <summary>🎥 View Execution Video</summary>
                        <video controls preload="metadata">
                            <source src="{}" type="video/mp4">
                            Your browser does not support the video tag.
                        </video>
                    </details>
                </div>
            "#,
                path
            )
        } else {
            String::new()
        };

        flows_html.push_str(&format!(
            r#"
            <div class="flow {flow_status_class}">
                <div class="flow-header">
                    <h3>{} <span class="flow-status-badge">{flow_status_text}</span></h3>
                    {duration_html}
                </div>
                <div class="commands">
                    {commands_html}
                </div>
                {video_html}
            </div>
            </div>
        "#,
            html_escape(&flow.flow_name),
            video_html = video_html
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Test Report - {}</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-primary: #0a0f1d;
            --bg-secondary: #141b2d;
            --bg-tertiary: #1f2937;
            --border: #374151;
            --text-primary: #f9fafb;
            --text-secondary: #9ca3af;
            --green: #10b981;
            --red: #ef4444;
            --yellow: #f59e0b;
            --blue: #3b82f6;
            --purple: #8b5cf6;
            --glass: rgba(255, 255, 255, 0.03);
        }}
        
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Inter', system-ui, -apple-system, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.5;
            padding: 3rem 1rem;
        }}
        
        .container {{
            max-width: 1100px;
            margin: 0 auto;
        }}
        
        header {{
            margin-bottom: 3rem;
            display: flex;
            justify-content: space-between;
            align-items: flex-end;
        }}
        
        h1 {{
            font-size: 2.25rem;
            font-weight: 800;
            letter-spacing: -0.025em;
            background: linear-gradient(135deg, #fff 0%, #94a3b8 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1.5rem;
            margin-bottom: 3rem;
        }}
        
        .stat {{
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            padding: 1.5rem;
            border-radius: 1rem;
            position: relative;
            overflow: hidden;
            transition: transform 0.2s;
        }}
        
        .stat:hover {{
            transform: translateY(-2px);
        }}
        
        .stat-value {{
            font-size: 2.5rem;
            font-weight: 800;
            margin-bottom: 0.25rem;
        }}
        
        .stat-label {{
            color: var(--text-secondary);
            font-size: 0.875rem;
            font-weight: 500;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        
        .stat.passed .stat-value {{ color: var(--green); }}
        .stat.failed .stat-value {{ color: var(--red); }}
        .stat.skipped .stat-value {{ color: var(--yellow); }}
        
        .progress-container {{
            margin-bottom: 4rem;
        }}
        
        .progress-bar {{
            background: var(--bg-secondary);
            height: 12px;
            border-radius: 6px;
            overflow: hidden;
            display: flex;
            border: 1px solid var(--border);
        }}
        
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, var(--green), #34d399);
            transition: width 0.8s cubic-bezier(0.16, 1, 0.3, 1);
        }}
        
        .flow {{
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            border-radius: 1.25rem;
            margin-bottom: 2rem;
            overflow: hidden;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }}
        
        .flow-header {{
            padding: 1.5rem;
            background: var(--glass);
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
        }}
        
        .flow-header h3 {{
            font-size: 1.25rem;
            font-weight: 700;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        
        .flow-status-badge {{
            padding: 0.25rem 0.75rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
        }}
        
        .flow.passed .flow-status-badge {{ background: rgba(16, 185, 129, 0.1); color: var(--green); }}
        .flow.failed .flow-status-badge {{ background: rgba(239, 68, 68, 0.1); color: var(--red); }}
        
        .commands {{
            padding: 1rem 1.5rem;
        }}
        
        .command {{
            padding: 1rem;
            border-radius: 0.75rem;
            display: flex;
            align-items: flex-start;
            gap: 1rem;
            margin-bottom: 0.5rem;
            transition: background 0.2s;
            cursor: pointer;
        }}
        
        .command:hover {{
            background: var(--bg-tertiary);
        }}
        
        .command-icon {{
            width: 2rem;
            height: 2rem;
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 0.5rem;
            font-size: 1.25rem;
            flex-shrink: 0;
        }}
        
        .command.passed .command-icon {{ background: rgba(16, 185, 129, 0.1); color: var(--green); }}
        .command.failed .command-icon {{ background: rgba(239, 68, 68, 0.1); color: var(--red); }}
        .command.skipped .command-icon {{ background: rgba(245, 158, 11, 0.1); color: var(--yellow); }}
        
        .command-content {{
            flex: 1;
        }}
        
        .command-name {{
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9375rem;
            font-weight: 500;
            color: var(--text-primary);
        }}
        
        .command-meta {{
            display: flex;
            gap: 1rem;
            margin-top: 0.25rem;
        }}
        
        .duration {{
            color: var(--text-secondary);
            font-size: 0.75rem;
            font-weight: 500;
        }}
        
        .screenshot-link {{
            color: var(--blue);
            font-size: 0.75rem;
            font-weight: 600;
            text-decoration: none;
            display: flex;
            align-items: center;
            gap: 0.25rem;
        }}
        
        .screenshot-link:hover {{
            text-decoration: underline;
        }}
        
        .error-message {{
            background: rgba(239, 68, 68, 0.1);
            border-radius: 0.5rem;
            padding: 0.75rem;
            margin-top: 0.75rem;
            color: #fca5a5;
            font-size: 0.8125rem;
            font-family: 'JetBrains Mono', monospace;
            border: 1px solid rgba(239, 68, 68, 0.2);
        }}
        
        .meta {{
            margin-top: 4rem;
            padding-top: 2rem;
            border-top: 1px solid var(--border);
            color: var(--text-secondary);
            font-size: 0.875rem;
            text-align: center;
            display: flex;
            justify-content: center;
            gap: 2rem;
        }}
        
        /* Modal */
        #modal {{
            display: none;
            position: fixed;
            z-index: 100;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.9);
            padding: 2rem;
            align-items: center;
            justify-content: center;
        }}
        
        #modal img {{
            max-width: 100%;
            max-height: 100%;
            border-radius: 0.5rem;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
        }}
        
        #modal.active {{
            display: flex;
        }}
        
        .video-details {{
            margin: 0rem 1.5rem 1rem 1.5rem;
            padding: 1rem;
            background: rgba(0, 0, 0, 0.2);
            border-radius: 0.75rem;
            border: 1px solid var(--border);
        }}
        
        .video-details summary {{
            cursor: pointer;
            font-weight: 600;
            color: var(--blue);
            outline: none;
            user-select: none;
            list-style: none; /* Hide default triangle */
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .video-details summary::-webkit-details-marker {{
            display: none;
        }}
        
        .video-details video {{
            margin-top: 1rem;
            border-radius: 0.5rem;
            width: 100%;
            max-width: 800px;
            display: block;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
            background: #000;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div>
                <div style="font-size: 0.875rem; font-weight: 600; color: var(--purple); text-transform: uppercase; letter-spacing: 0.1em; margin-bottom: 0.5rem;">Automated Testing</div>
                <h1>Test Execution Report</h1>
            </div>
            <div style="text-align: right;">
                <div style="font-size: 0.875rem; color: var(--text-secondary);">Session Duration</div>
                <div style="font-size: 1.25rem; font-weight: 700;">{}</div>
            </div>
        </header>
        
        <div class="summary">
            <div class="stat">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Flows</div>
            </div>
            <div class="stat">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Commands</div>
            </div>
            <div class="stat passed">
                <div class="stat-value">{}</div>
                <div class="stat-label">Passed</div>
            </div>
            <div class="stat failed">
                <div class="stat-value">{}</div>
                <div class="stat-label">Failed</div>
            </div>
        </div>
        
        <div class="progress-container">
            <div style="display: flex; justify-content: space-between; margin-bottom: 0.75rem;">
                <span style="font-weight: 600; font-size: 0.875rem;">Success Rate</span>
                <span style="font-weight: 700; color: var(--green);">{pass_rate}%</span>
            </div>
            <div class="progress-bar">
                <div class="progress-fill" style="width: {pass_rate}%"></div>
            </div>
        </div>
        
        {flows_html}
        
        <div class="meta">
            <span>Session: {}</span>
            <span>Generated: {}</span>
        </div>
    </div>

    <div id="modal" onclick="this.classList.remove('active')">
        <img id="modal-img" src="" alt="Screenshot">
    </div>

    <script>
        function showScreenshot(path) {{
            const modal = document.getElementById('modal');
            const img = document.getElementById('modal-img');
            img.src = path;
            modal.classList.add('active');
            event.stopPropagation();
        }}
    </script>
</body>
</html>"#,
        summary.session_id,
        format_duration(summary.total_duration_ms.unwrap_or(0)),
        summary.total_flows,
        summary.total_commands,
        summary.passed,
        summary.failed,
        summary.session_id,
        results.generated_at
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) as f64 / 1000.0;
        format!("{}m {:.0}s", minutes, seconds)
    }
}
