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

    // Calculate stability matrix per flow
    let mut flow_stats: std::collections::HashMap<String, (usize, usize, usize, u64)> = std::collections::HashMap::new();
    for flow in &results.flows {
        let entry = flow_stats.entry(flow.flow_name.clone()).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        match flow.status {
            FlowStatus::Passed => entry.1 += 1,
            FlowStatus::Failed | FlowStatus::PartiallyPassed { .. } => entry.2 += 1,
            _ => {}
        }
        entry.3 += flow.total_duration_ms.unwrap_or(0);
    }

    let mut stability_table_html = String::new();
    if results.flows.len() > 1 || flow_stats.values().any(|(total, _, _, _)| *total > 1) {
        let mut rows = String::new();
        let mut sorted_stats: Vec<_> = flow_stats.into_iter().collect();
        sorted_stats.sort_by(|a, b| a.0.cmp(&b.0));

        for (flow_name, (total, passed, failed, duration_ms)) in sorted_stats {
            let flow_pass_rate = if total > 0 { (passed as f64 / total as f64 * 100.0) as u32 } else { 0 };
            let (status_badge, status_class) = if failed == 0 {
                ("STABLE (100%)", "passed")
            } else if passed > 0 {
                ("FLAKY", "flaky")
            } else {
                ("FAILING", "failed")
            };

            rows.push_str(&format!(
                r#"
                <tr>
                    <td><span class="flow-pill">{}</span></td>
                    <td><strong>{}</strong> run(s)</td>
                    <td><span style="color: var(--green); font-weight: 600;">{} passed</span> / <span style="color: var(--red); font-weight: 600;">{} failed</span></td>
                    <td><strong>{}%</strong></td>
                    <td>{}</td>
                    <td><span class="status-badge {}">{}</span></td>
                </tr>
                "#,
                html_escape(&flow_name),
                total,
                passed,
                failed,
                flow_pass_rate,
                format_duration(duration_ms),
                status_class,
                status_badge
            ));
        }

        stability_table_html = format!(
            r#"
            <div class="stability-card">
                <div class="card-header">
                    <h3>📊 Flow Execution & Stability Matrix</h3>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Flow Name</th>
                            <th>Total Runs</th>
                            <th>Passed / Failed</th>
                            <th>Pass Rate</th>
                            <th>Total Time</th>
                            <th>Stability</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
            "#,
            rows
        );
    }

    let mut flows_html = String::new();
    for (flow_idx, flow) in results.flows.iter().enumerate() {
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

            let retry_html = if cmd.retry_count > 0 {
                format!(r#"<span class="retry-badge">↻ Retried {} time(s)</span>"#, cmd.retry_count)
            } else {
                String::new()
            };

            let duration_html = cmd
                .duration_ms
                .map(|d| format!("<span class=\"duration\">{}ms</span>", d))
                .unwrap_or_default();

            // Rich failure inspector with inline thumbnail, XML hierarchy and recent logs
            let error_html = match &cmd.status {
                CommandStatus::Failed { error } => {
                    let screenshot_box = if let Some(path) = &cmd.screenshot_path {
                        let img_src = if let Ok(bytes) = std::fs::read(path) {
                            use base64::{engine::general_purpose::STANDARD, Engine};
                            format!("data:image/png;base64,{}", STANDARD.encode(&bytes))
                        } else {
                            std::path::Path::new(path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(path)
                                .to_string()
                        };

                        format!(
                            r#"<div class="evidence-thumb-box" onclick="showScreenshot(this.querySelector('img').src)">
                                <img src="{img_src}" alt="Failure Snapshot" />
                                <div class="zoom-label">🔍 Zoom Screenshot</div>
                            </div>"#,
                            img_src = img_src
                        )
                    } else {
                        String::new()
                    };

                    let hierarchy_box = if let Some(path) = &cmd.ui_hierarchy_path {
                        let snippet = match std::fs::read_to_string(path) {
                            Ok(content) => {
                                let lines: Vec<&str> = content.lines().collect();
                                if lines.len() > 60 {
                                    format!("{}\n<!-- ... ({} more lines) ... -->", lines[..60].join("\n"), lines.len() - 60)
                                } else {
                                    content
                                }
                            }
                            Err(_) => format!("Path: {}", path),
                        };
                        format!(
                            r#"<details class="evidence-details">
                                <summary>📄 View UI Hierarchy XML ({path})</summary>
                                <pre><code>{snippet}</code></pre>
                            </details>"#,
                            path = path,
                            snippet = html_escape(&snippet)
                        )
                    } else {
                        String::new()
                    };

                    let logs_box = if let Some(path) = &cmd.log_path {
                        let snippet = match std::fs::read_to_string(path) {
                            Ok(content) => {
                                let lines: Vec<&str> = content.lines().collect();
                                if lines.len() > 50 {
                                    lines[lines.len() - 50..].join("\n")
                                } else {
                                    content
                                }
                            }
                            Err(_) => format!("Path: {}", path),
                        };
                        format!(
                            r#"<details class="evidence-details">
                                <summary>📋 View Recent Device Logs ({path})</summary>
                                <pre><code>{snippet}</code></pre>
                            </details>"#,
                            path = path,
                            snippet = html_escape(&snippet)
                        )
                    } else {
                        String::new()
                    };

                    format!(
                        r##"
                        <div class="failure-inspector">
                            <div class="error-header">
                                <span class="error-badge">FAILURE DETAILS</span>
                                <span class="cmd-idx">Step #{}</span>
                            </div>
                            <div class="error-message">{}</div>
                            {}
                            <div class="evidence-grid">
                                {}
                                <div class="evidence-texts">
                                    {}
                                    {}
                                </div>
                            </div>
                        </div>
                        "##,
                        cmd.index,
                        html_escape(error),
                        camera_hint_html(error),
                        screenshot_box,
                        hierarchy_box,
                        logs_box
                    )
                }
                _ => String::new(),
            };

            commands_html.push_str(&format!(
                r##"
                <div class="command {status_class}">
                    <div class="command-icon">{status_icon}</div>
                    <div class="command-content">
                        <div class="command-title-row">
                            <span class="command-name">[{}] {}</span>
                            <div class="command-meta">
                                {retry_html}
                                {duration_html}
                            </div>
                        </div>
                        {error_html}
                    </div>
                </div>
            "##,
                cmd.index,
                html_escape(&cmd.command_display),
                status_class = status_class,
                status_icon = status_icon,
                retry_html = retry_html,
                duration_html = duration_html,
                error_html = error_html
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
                    <h3>#{run_num}: {} <span class="flow-status-badge">{flow_status_text}</span></h3>
                    {duration_html}
                </div>
                <div class="commands">
                    {commands_html}
                </div>
                {video_html}
            </div>
        "#,
            html_escape(&flow.flow_name),
            run_num = flow_idx + 1,
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
        
        .command-title-row {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 0.5rem;
        }}

        .retry-badge {{
            background: rgba(245, 158, 11, 0.15);
            color: var(--yellow);
            padding: 0.15rem 0.5rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 700;
        }}

        .stability-card {{
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            border-radius: 1rem;
            margin-bottom: 2.5rem;
            overflow: hidden;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }}

        .card-header {{
            padding: 1rem 1.5rem;
            background: var(--glass);
            border-bottom: 1px solid var(--border);
        }}

        .card-header h3 {{
            font-size: 1.125rem;
            font-weight: 700;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            text-align: left;
        }}

        th {{
            padding: 0.875rem 1.25rem;
            font-size: 0.75rem;
            font-weight: 700;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            border-bottom: 1px solid var(--border);
            background: var(--glass);
        }}

        td {{
            padding: 0.875rem 1.25rem;
            border-bottom: 1px solid rgba(51, 65, 85, 0.4);
            font-size: 0.875rem;
        }}

        .status-badge {{
            display: inline-block;
            padding: 0.2rem 0.5rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 700;
            text-transform: uppercase;
        }}
        .status-badge.passed {{ background: rgba(16, 185, 129, 0.15); color: var(--green); }}
        .status-badge.failed {{ background: rgba(239, 68, 68, 0.15); color: var(--red); }}
        .status-badge.flaky {{ background: rgba(245, 158, 11, 0.15); color: var(--yellow); }}

        .flow-pill {{
            display: inline-block;
            background: rgba(139, 92, 246, 0.15);
            color: var(--purple);
            padding: 0.2rem 0.5rem;
            border-radius: 0.375rem;
            font-weight: 600;
            font-size: 0.8125rem;
        }}

        /* Failure Inspector Styles */
        .failure-inspector {{
            background: rgba(239, 68, 68, 0.07);
            border: 1px solid rgba(239, 68, 68, 0.25);
            border-radius: 0.75rem;
            padding: 1rem;
            margin-top: 0.75rem;
        }}

        .error-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }}

        .error-badge {{
            font-size: 0.75rem;
            font-weight: 800;
            color: #ef4444;
            letter-spacing: 0.05em;
        }}

        .cmd-idx {{
            font-size: 0.75rem;
            color: var(--text-secondary);
            font-family: 'JetBrains Mono', monospace;
        }}

        .evidence-grid {{
            display: grid;
            grid-template-columns: auto 1fr;
            gap: 1rem;
            margin-top: 0.75rem;
            align-items: flex-start;
        }}

        @media (max-width: 768px) {{
            .evidence-grid {{
                grid-template-columns: 1fr;
            }}
        }}

        .evidence-thumb-box {{
            position: relative;
            cursor: pointer;
            border-radius: 0.5rem;
            overflow: hidden;
            border: 1px solid var(--border);
            max-width: 200px;
            background: #000;
        }}

        .evidence-thumb-box img {{
            width: 100%;
            height: auto;
            display: block;
            transition: transform 0.2s;
        }}

        .evidence-thumb-box:hover img {{
            transform: scale(1.03);
        }}

        .zoom-label {{
            position: absolute;
            bottom: 0;
            left: 0;
            right: 0;
            background: rgba(0, 0, 0, 0.7);
            color: #fff;
            font-size: 0.6875rem;
            padding: 0.2rem;
            text-align: center;
        }}

        .evidence-texts {{
            flex: 1;
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }}

        .evidence-details {{
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            padding: 0.5rem 0.75rem;
            font-size: 0.8125rem;
        }}

        .evidence-details summary {{
            cursor: pointer;
            color: var(--blue);
            font-weight: 600;
            user-select: none;
            outline: none;
        }}

        .evidence-details pre {{
            margin-top: 0.5rem;
            padding: 0.5rem;
            background: #000;
            border-radius: 0.375rem;
            overflow-x: auto;
            max-height: 250px;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.75rem;
            color: #94a3b8;
            white-space: pre-wrap;
            word-break: break-all;
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

        .camera-hint {{
            background: rgba(59, 130, 246, 0.1);
            border: 1px solid rgba(59, 130, 246, 0.25);
            border-radius: 0.5rem;
            padding: 0.75rem;
            margin-top: 0.75rem;
            color: #bfdbfe;
            font-size: 0.8125rem;
        }}

        .camera-hint pre {{
            white-space: pre-wrap;
            margin-top: 0.5rem;
            font-family: 'JetBrains Mono', monospace;
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

        {stability_table_html}
        
        {flows_html}
        
        <div class="meta">
            <span>Session: {}</span>
            <span>Generated: {}</span>
        </div>
    </div>

    <div id="modal" onclick="this.classList.remove('active')">
        <div style="position: relative; max-width: 90vw; max-height: 90vh; display: flex; flex-direction: column; align-items: center;" onclick="event.stopPropagation()">
            <img id="modal-img" src="" alt="Screenshot" style="max-width: 100%; max-height: 80vh; object-fit: contain; border-radius: 0.5rem; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7);">
            <div style="margin-top: 0.75rem; color: #94a3b8; font-size: 0.8125rem; background: rgba(0,0,0,0.6); padding: 0.25rem 0.75rem; border-radius: 9999px; cursor: pointer;" onclick="document.getElementById('modal').classList.remove('active')">✕ Close Preview</div>
        </div>
    </div>

    <script>
        function showScreenshot(src) {{
            const modal = document.getElementById('modal');
            const img = document.getElementById('modal-img');
            img.src = src;
            modal.classList.add('active');
            if (window.event) {{
                window.event.stopPropagation();
            }}
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
        results.generated_at,
        pass_rate = pass_rate,
        stability_table_html = stability_table_html,
        flows_html = flows_html
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn camera_hint_html(error: &str) -> String {
    let Some(hint) = crate::camera::launcher::camera_failure_hint(error) else {
        return String::new();
    };

    format!(
        r#"<div class="camera-hint"><strong>Camera next steps</strong><pre>{}</pre></div>"#,
        html_escape(&hint)
    )
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


/// Item summary for test sessions dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionDashboardItem {
    pub session_id: String,
    pub target_name: String,
    pub created_at: String,
    pub total_flows: usize,
    pub total_commands: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub report_path: String,
    pub is_passed: bool,
}

/// Generate Sessions History Dashboard HTML (output/index.html and output/sessions/index.html)
pub fn generate_sessions_dashboard(output_dir: &Path) -> Result<()> {
    let sessions_dir = output_dir.join("sessions");
    if !sessions_dir.exists() {
        return Ok(());
    }

    let mut sessions: Vec<SessionDashboardItem> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let session_path = entry.path();
            if !session_path.is_dir() {
                continue;
            }

            let session_id = entry.file_name().to_string_lossy().to_string();
            let session_info_path = session_path.join("session.json");

            if session_info_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&session_info_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let summary = val.get("summary");
                        let total_flows = summary
                            .and_then(|s| s.get("totalFlows").or_else(|| s.get("total_flows")))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let total_commands = summary
                            .and_then(|s| s.get("totalCommands").or_else(|| s.get("total_commands")))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let passed = summary
                            .and_then(|s| s.get("passed"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let failed = summary
                            .and_then(|s| s.get("failed"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let duration_ms = summary
                            .and_then(|s| s.get("totalDurationMs").or_else(|| s.get("total_duration_ms")))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let created_at = val
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let target_name = session_id
                            .strip_prefix("session_")
                            .and_then(|s| {
                                if let Some(idx) = s.rfind('_') {
                                    let before_time = &s[..idx];
                                    if let Some(date_idx) = before_time.rfind('_') {
                                        let candidate_date = &before_time[date_idx + 1..];
                                        if candidate_date.len() == 10 && candidate_date.contains('-') {
                                            return Some(before_time[..date_idx].to_string());
                                        }
                                    }
                                }
                                None
                            })
                            .unwrap_or_else(|| {
                                if total_flows == 1 {
                                    "flow".to_string()
                                } else {
                                    "suite".to_string()
                                }
                            });

                        let rel_report = format!("sessions/{}/report/report.html", session_id);
                        let is_passed = failed == 0 && (passed > 0 || total_commands > 0);

                        sessions.push(SessionDashboardItem {
                            session_id,
                            target_name,
                            created_at,
                            total_flows,
                            total_commands,
                            passed,
                            failed,
                            duration_ms,
                            report_path: rel_report,
                            is_passed,
                        });
                    }
                }
            }
        }
    }

    sessions.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.session_id.cmp(&a.session_id))
    });

    let html = render_sessions_dashboard_html(&sessions, false);
    let index_path = output_dir.join("index.html");
    std::fs::write(&index_path, &html)?;

    let sessions_index_path = sessions_dir.join("index.html");
    let sessions_html = render_sessions_dashboard_html(&sessions, true);
    let _ = std::fs::write(&sessions_index_path, sessions_html);

    Ok(())
}

fn render_sessions_dashboard_html(sessions: &[SessionDashboardItem], inside_sessions_dir: bool) -> String {
    let total_sessions = sessions.len();
    let passed_sessions = sessions.iter().filter(|s| s.is_passed).count();
    let failed_sessions = total_sessions.saturating_sub(passed_sessions);
    let pass_rate = if total_sessions > 0 {
        (passed_sessions as f64 / total_sessions as f64 * 100.0) as u32
    } else {
        0
    };

    // Calculate aggregated stability per flow/target across all sessions
    let mut flow_stats: std::collections::HashMap<String, (usize, usize, usize, u64)> = std::collections::HashMap::new();
    for item in sessions {
        let entry = flow_stats.entry(item.target_name.clone()).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        if item.is_passed {
            entry.1 += 1;
        } else {
            entry.2 += 1;
        }
        entry.3 += item.duration_ms;
    }

    let mut stability_section_html = String::new();
    if !flow_stats.is_empty() {
        let mut rows = String::new();
        let mut sorted: Vec<_> = flow_stats.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        for (name, (total, passed, failed, duration_ms)) in sorted {
            let rate = if total > 0 { (passed as f64 / total as f64 * 100.0) as u32 } else { 0 };
            let (status_badge, status_class) = if failed == 0 {
                ("STABLE (100%)", "passed")
            } else if passed > 0 {
                ("FLAKY", "flaky")
            } else {
                ("FAILING", "failed")
            };

            rows.push_str(&format!(
                r#"
                <tr>
                    <td><span class="flow-pill">{}</span></td>
                    <td><strong>{}</strong> session(s)</td>
                    <td><span style="color: var(--green); font-weight: 600;">{} passed</span> / <span style="color: var(--red); font-weight: 600;">{} failed</span></td>
                    <td><strong>{}%</strong></td>
                    <td>{}</td>
                    <td><span class="status-badge {}">{}</span></td>
                </tr>
                "#,
                html_escape(&name),
                total,
                passed,
                failed,
                rate,
                format_duration(duration_ms),
                status_class,
                status_badge
            ));
        }

        stability_section_html = format!(
            r#"
            <div class="table-card" style="margin-bottom: 2rem;">
                <div style="padding: 1rem 1.25rem; background: var(--glass); border-bottom: 1px solid var(--border); font-weight: 700;">
                    📈 Flow Reliability & Flakiness Breakdown
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Flow / Target</th>
                            <th>Total Sessions</th>
                            <th>Passed / Failed</th>
                            <th>Pass Rate</th>
                            <th>Total Duration</th>
                            <th>Reliability Status</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
            "#,
            rows
        );
    }

    let mut rows_html = String::new();
    for item in sessions {
        let (status_badge, status_class) = if item.is_passed {
            ("PASSED", "passed")
        } else {
            ("FAILED", "failed")
        };

        let link_path = if inside_sessions_dir {
            format!("{}/report/report.html", item.session_id)
        } else {
            item.report_path.clone()
        };

        let duration_text = format_duration(item.duration_ms);

        rows_html.push_str(&format!(
            r##"
            <tr class="session-row {status_class}" data-status="{status_class}" data-search="{search_text}">
                <td><span class="status-badge {status_class}">{status_badge}</span></td>
                <td class="target-name">
                    <span class="flow-pill">{target_name}</span>
                    <div class="session-id">{session_id}</div>
                </td>
                <td class="date-cell">{created_at}</td>
                <td>{total_flows} flow(s), {total_commands} cmd(s)</td>
                <td>{passed} pass / {failed} fail</td>
                <td class="duration-cell">{duration_text}</td>
                <td class="action-cell">
                    <a href="{link_path}" class="btn-view">View Report ↗</a>
                </td>
            </tr>
            "##,
            status_class = status_class,
            status_badge = status_badge,
            target_name = html_escape(&item.target_name),
            session_id = html_escape(&item.session_id),
            created_at = html_escape(&item.created_at),
            total_flows = item.total_flows,
            total_commands = item.total_commands,
            passed = item.passed,
            failed = item.failed,
            duration_text = duration_text,
            link_path = link_path,
            search_text = html_escape(&format!("{} {} {}", item.session_id, item.target_name, item.created_at).to_lowercase())
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lumi Tester - Test Sessions Dashboard</title>
    <style>
        :root {{
            --bg-primary: #0f172a;
            --bg-secondary: #1e293b;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --border: #334155;
            --green: #10b981;
            --red: #ef4444;
            --yellow: #f59e0b;
            --purple: #8b5cf6;
            --blue: #3b82f6;
            --glass: rgba(30, 41, 59, 0.7);
        }}
        * {{ margin: 0; padding: 0; box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; }}
        body {{ background-color: var(--bg-primary); color: var(--text-primary); padding: 2rem; min-height: 100vh; }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem; }}
        .logo-tag {{ font-size: 0.8125rem; font-weight: 700; color: var(--purple); text-transform: uppercase; letter-spacing: 0.1em; margin-bottom: 0.25rem; }}
        h1 {{ font-size: 2rem; font-weight: 800; letter-spacing: -0.025em; }}
        
        .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1.25rem; margin-bottom: 2rem; }}
        .stat {{ background: var(--bg-secondary); border: 1px solid var(--border); padding: 1.25rem; border-radius: 0.875rem; }}
        .stat-value {{ font-size: 2rem; font-weight: 800; margin-bottom: 0.25rem; }}
        .stat-label {{ color: var(--text-secondary); font-size: 0.8125rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }}
        .stat.passed .stat-value {{ color: var(--green); }}
        .stat.failed .stat-value {{ color: var(--red); }}

        .toolbar {{ display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap; }}
        .filter-group {{ display: flex; gap: 0.5rem; }}
        .filter-btn {{ background: var(--bg-secondary); border: 1px solid var(--border); color: var(--text-secondary); padding: 0.5rem 1rem; border-radius: 0.5rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s; }}
        .filter-btn.active {{ background: var(--purple); color: #fff; border-color: var(--purple); }}
        .search-box {{ flex: 1; max-width: 380px; position: relative; }}
        .search-box input {{ width: 100%; background: var(--bg-secondary); border: 1px solid var(--border); color: var(--text-primary); padding: 0.5rem 1rem; border-radius: 0.5rem; font-size: 0.875rem; outline: none; }}
        .search-box input:focus {{ border-color: var(--purple); }}

        .table-card {{ background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 1rem; overflow: hidden; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1); }}
        table {{ width: 100%; border-collapse: collapse; text-align: left; }}
        th {{ padding: 1rem 1.25rem; font-size: 0.75rem; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; border-bottom: 1px solid var(--border); background: var(--glass); }}
        td {{ padding: 1rem 1.25rem; border-bottom: 1px solid rgba(51, 65, 85, 0.4); font-size: 0.875rem; }}
        tr:last-child td {{ border-bottom: none; }}
        tr.session-row:hover {{ background: rgba(51, 65, 85, 0.3); }}

        .status-badge {{ display: inline-block; padding: 0.25rem 0.625rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; }}
        .status-badge.passed {{ background: rgba(16, 185, 129, 0.15); color: var(--green); }}
        .status-badge.failed {{ background: rgba(239, 68, 68, 0.15); color: var(--red); }}

        .flow-pill {{ display: inline-block; background: rgba(139, 92, 246, 0.15); color: var(--purple); padding: 0.2rem 0.5rem; border-radius: 0.375rem; font-weight: 600; font-size: 0.8125rem; margin-bottom: 0.2rem; }}
        .session-id {{ font-size: 0.75rem; color: var(--text-secondary); font-family: monospace; }}
        .date-cell {{ color: var(--text-secondary); font-size: 0.8125rem; }}
        .duration-cell {{ font-weight: 600; color: #cbd5e1; }}
        
        .btn-view {{ display: inline-block; background: var(--purple); color: #fff; text-decoration: none; padding: 0.375rem 0.75rem; border-radius: 0.375rem; font-weight: 600; font-size: 0.8125rem; transition: opacity 0.2s; }}
        .btn-view:hover {{ opacity: 0.85; }}

        .empty-state {{ text-align: center; padding: 3rem; color: var(--text-secondary); }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div>
                <div class="logo-tag">Lumi Tester</div>
                <h1>Test Sessions Dashboard</h1>
            </div>
            <div>
                <a href="report.html" class="btn-view" style="padding: 0.5rem 1rem; font-size: 0.875rem;">📊 Open Latest Report ↗</a>
            </div>
        </header>

        <div class="summary">
            <div class="stat">
                <div class="stat-value">{total_sessions}</div>
                <div class="stat-label">Total Sessions</div>
            </div>
            <div class="stat passed">
                <div class="stat-value">{passed_sessions}</div>
                <div class="stat-label">Passed</div>
            </div>
            <div class="stat failed">
                <div class="stat-value">{failed_sessions}</div>
                <div class="stat-label">Failed</div>
            </div>
            <div class="stat">
                <div class="stat-value" style="color: var(--purple);">{pass_rate}%</div>
                <div class="stat-label">Success Rate</div>
            </div>
        </div>

        {stability_section_html}

        <div class="toolbar">
            <div class="filter-group">
                <button class="filter-btn active" onclick="setFilter('all', this)">All ({total_sessions})</button>
                <button class="filter-btn" onclick="setFilter('passed', this)">Passed ({passed_sessions})</button>
                <button class="filter-btn" onclick="setFilter('failed', this)">Failed ({failed_sessions})</button>
            </div>
            <div class="search-box">
                <input type="text" id="searchInput" placeholder="Search by session name, flow, date..." onkeyup="filterRows()">
            </div>
        </div>

        <div class="table-card">
            <table>
                <thead>
                    <tr>
                        <th>Status</th>
                        <th>Target & Session ID</th>
                        <th>Executed At</th>
                        <th>Execution Counts</th>
                        <th>Results</th>
                        <th>Duration</th>
                        <th>Action</th>
                    </tr>
                </thead>
                <tbody id="sessionTableBody">
                    {rows_html}
                </tbody>
            </table>
        </div>
    </div>

    <script>
        let currentFilter = 'all';

        function setFilter(status, btn) {{
            currentFilter = status;
            document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            filterRows();
        }}

        function filterRows() {{
            const search = document.getElementById('searchInput').value.toLowerCase();
            const rows = document.querySelectorAll('.session-row');
            
            rows.forEach(row => {{
                const status = row.getAttribute('data-status');
                const text = row.getAttribute('data-search');
                
                const matchesFilter = (currentFilter === 'all' || status === currentFilter);
                const matchesSearch = text.includes(search);
                
                if (matchesFilter && matchesSearch) {{
                    row.style.display = '';
                }} else {{
                    row.style.display = 'none';
                }}
            }});
        }}
    </script>
</body>
</html>"##,
        total_sessions = total_sessions,
        passed_sessions = passed_sessions,
        failed_sessions = failed_sessions,
        pass_rate = pass_rate,
        stability_section_html = stability_section_html,
        rows_html = rows_html
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_failure_hint_is_rendered_as_html() {
        let error = "device check failed: button 'device_2.button_1' is 'UNKNOWN', expected 'BLUE'\ncamera evidence: output/camera_evidence/default_123";

        let html = camera_hint_html(error);

        assert!(html.contains("Camera next steps"));
        assert!(html.contains("lumi-tester camera profile"));
        assert!(html.contains("device_2.button_1"));
    }

    #[test]
    fn test_render_sessions_dashboard_html() {
        let items = vec![
            SessionDashboardItem {
                session_id: "session_slider_2026-08-22_10-04-45".to_string(),
                target_name: "slider".to_string(),
                created_at: "2026-08-22 10:04:45".to_string(),
                total_flows: 1,
                total_commands: 3,
                passed: 3,
                failed: 0,
                duration_ms: 1500,
                report_path: "sessions/session_slider_2026-08-22_10-04-45/report/report.html".to_string(),
                is_passed: true,
            },
            SessionDashboardItem {
                session_id: "session_login_2026-08-22_09-15-30".to_string(),
                target_name: "login".to_string(),
                created_at: "2026-08-22 09:15:30".to_string(),
                total_flows: 1,
                total_commands: 5,
                passed: 4,
                failed: 1,
                duration_ms: 2400,
                report_path: "sessions/session_login_2026-08-22_09-15-30/report/report.html".to_string(),
                is_passed: false,
            },
        ];

        let html = render_sessions_dashboard_html(&items, false);
        assert!(html.contains("Lumi Tester - Test Sessions Dashboard"));
        assert!(html.contains("slider"));
        assert!(html.contains("login"));
        assert!(html.contains("PASSED"));
        assert!(html.contains("FAILED"));
    }

    #[test]
    fn test_generate_sessions_dashboard_from_dir() {
        let temp_dir = std::env::temp_dir().join(format!("lumi_dash_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        let sessions_dir = temp_dir.join("sessions");
        let session1_dir = sessions_dir.join("session_login_2026-08-22_10-00-00");
        std::fs::create_dir_all(&session1_dir).unwrap();

        let session_json = serde_json::json!({
            "created_at": "2026-08-22 10:00:00",
            "session_id": "session_login_2026-08-22_10-00-00",
            "summary": {
                "failed": 0,
                "passed": 5,
                "sessionId": "session_login_2026-08-22_10-00-00",
                "skipped": 0,
                "totalCommands": 5,
                "totalDurationMs": 1200,
                "totalFlows": 1
            }
        });
        std::fs::write(session1_dir.join("session.json"), session_json.to_string()).unwrap();

        assert!(generate_sessions_dashboard(&temp_dir).is_ok());

        let index_content = std::fs::read_to_string(temp_dir.join("index.html")).unwrap();
        assert!(index_content.contains("PASSED"));
        assert!(index_content.contains("5 cmd(s)"));
        assert!(index_content.contains("5 pass / 0 fail"));
        assert!(index_content.contains("1.2s"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

