#!/usr/bin/env python3
"""
Local Terraform Deployment UI

A simple web interface for configuring and deploying Terraform templates locally.
This script is included with downloaded templates to provide an easy deployment experience.
Supports both Azure and AWS templates.
"""

import os
import subprocess
import json
import threading
import sys
from flask import Flask, render_template_string, request, redirect, url_for, flash
from werkzeug.utils import secure_filename
try:
    import webview
    HAS_WEBVIEW = True
except ImportError:
    HAS_WEBVIEW = False

app = Flask(__name__)
app.secret_key = 'local-deployment-key'

# Auto-detect template path - look for azure-simple or aws-simple directories
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
TEMPLATE_DIRS = {
    'azure-simple': os.path.join(SCRIPT_DIR, 'templates', 'azure-simple'),
    'aws-simple': os.path.join(SCRIPT_DIR, 'templates', 'aws-simple')
}

# Allow template selection via command line argument
SELECTED_TEMPLATE = None
if len(sys.argv) > 1:
    arg_template = sys.argv[1].lower()
    if arg_template in TEMPLATE_DIRS:
        SELECTED_TEMPLATE = arg_template

# Detect which template is present
DETECTED_TEMPLATE = None
TEMPLATE_PATH = None

if SELECTED_TEMPLATE:
    # Use command-line specified template
    DETECTED_TEMPLATE = SELECTED_TEMPLATE
    TEMPLATE_PATH = TEMPLATE_DIRS[SELECTED_TEMPLATE]
    if not os.path.exists(TEMPLATE_PATH):
        print(f"ERROR: Template '{SELECTED_TEMPLATE}' not found at {TEMPLATE_PATH}")
        sys.exit(1)
else:
    # Auto-detect from available templates
    for template_name, template_path in TEMPLATE_DIRS.items():
        if os.path.exists(template_path) and os.path.exists(os.path.join(template_path, 'terraform.tfvars')):
            DETECTED_TEMPLATE = template_name
            TEMPLATE_PATH = template_path
            break

# If no template found, check if we're inside a template directory
if not TEMPLATE_PATH:
    if os.path.exists(os.path.join(SCRIPT_DIR, 'terraform.tfvars')):
        TEMPLATE_PATH = SCRIPT_DIR
        # Try to detect template type from files
        if os.path.exists(os.path.join(SCRIPT_DIR, 'azure.tf')):
            DETECTED_TEMPLATE = 'azure-simple'
        elif os.path.exists(os.path.join(SCRIPT_DIR, 'aws.tf')):
            DETECTED_TEMPLATE = 'aws-simple'
        else:
            DETECTED_TEMPLATE = 'unknown'

# Global variable to store deployment status
deploy_status = {'running': False, 'output': '', 'success': None, 'command': None}

# Global to store last command output
last_output = ''

def parse_tfvars(content):
    """Parse terraform.tfvars content into a dictionary"""
    data = {}
    lines = content.split('\n')
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if '=' in line and not line.startswith('#'):
            key, value = line.split('=', 1)
            key = key.strip()
            value = value.strip()
            if value.startswith('"') and value.endswith('"'):
                data[key] = value[1:-1]
            elif value == 'true':
                data[key] = True
            elif value == 'false':
                data[key] = False
            elif value.startswith('{'):
                # Parse map
                map_lines = [value]
                i += 1
                while i < len(lines) and not lines[i].strip().endswith('}'):
                    map_lines.append(lines[i])
                    i += 1
                if i < len(lines):
                    map_lines.append(lines[i])
                map_content = '\n'.join(map_lines)
                # Simple map parsing
                map_data = {}
                for mline in map_lines:
                    mline = mline.strip()
                    if ':' in mline and not mline.startswith('#'):
                        mkey, mval = mline.split(':', 1)
                        mkey = mkey.strip().strip('"')
                        mval = mval.strip().strip(',').strip('"')
                        map_data[mkey] = mval
                data[key] = map_data
            else:
                data[key] = value.strip('"')
        i += 1
    return data

def save_tfvars(data, tfvars_path):
    """Save data to terraform.tfvars file"""
    content = []
    for key, value in data.items():
        if isinstance(value, bool):
            content.append(f'{key} = {str(value).lower()}')
        elif isinstance(value, dict):
            content.append(f'{key} = {{')
            for k, v in value.items():
                content.append(f'  "{k}": "{v}"')
            content.append('}')
        else:
            content.append(f'{key} = "{value}"')

    with open(tfvars_path, 'w') as f:
        f.write('\n'.join(content))

def run_terraform_command(command, cwd):
    """Run a terraform command asynchronously"""
    global deploy_status
    deploy_status['running'] = True
    deploy_status['output'] = ''
    deploy_status['success'] = None
    deploy_status['command'] = ' '.join(command)

    try:
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            cwd=cwd
        )

        while True:
            output = process.stdout.readline()
            if output == '' and process.poll() is not None:
                break
            if output:
                deploy_status['output'] += output

        deploy_status['success'] = process.returncode == 0

    except Exception as e:
        deploy_status['output'] = str(e)
        deploy_status['success'] = False

    deploy_status['running'] = False
    global last_output
    last_output = deploy_status['output']

@app.route('/', methods=['GET', 'POST'])
def index():
    tfvars_path = os.path.join(TEMPLATE_PATH, 'terraform.tfvars')

    if request.method == 'POST':
        if 'update' in request.form:
            # Update tfvars
            data = parse_tfvars(open(tfvars_path).read())
            for key in request.form:
                if key != 'update' and key in data:
                    if isinstance(data[key], bool):
                        data[key] = request.form.get(key) == 'on'
                    elif isinstance(data[key], dict):
                        # For maps, assume JSON input
                        try:
                            data[key] = json.loads(request.form[key])
                        except:
                            pass
                    else:
                        data[key] = request.form[key]
            # Handle tags
            data['tags'] = {}
            tag_keys = [k for k in request.form if k.startswith('tags_key_')]
            for tk in tag_keys:
                index = tk.split('_')[-1]
                k = request.form[tk].strip()
                v = request.form.get(f'tags_value_{index}', '').strip()
                if k:
                    data['tags'][k] = v
            save_tfvars(data, tfvars_path)
            flash('Configuration updated successfully!', 'success')

        elif 'init' in request.form:
            # Autosave configuration
            if os.path.exists(tfvars_path):
                data = parse_tfvars(open(tfvars_path).read())
            else:
                data = {}
            for key in request.form:
                if key != 'init' and key in data:
                    if isinstance(data[key], bool):
                        data[key] = request.form.get(key) == 'on'
                    elif isinstance(data[key], dict):
                        # For maps, assume JSON input
                        try:
                            data[key] = json.loads(request.form[key])
                        except:
                            pass
                    else:
                        data[key] = request.form[key]
            # Handle tags
            data['tags'] = {}
            tag_keys = [k for k in request.form if k.startswith('tags_key_')]
            for tk in tag_keys:
                index = tk.split('_')[-1]
                k = request.form[tk].strip()
                v = request.form.get(f'tags_value_{index}', '').strip()
                if k:
                    data['tags'][k] = v
            save_tfvars(data, tfvars_path)
            flash('Configuration saved. Initializing Terraform...', 'success')
            threading.Thread(target=run_terraform_command, args=(['terraform', 'init'], TEMPLATE_PATH)).start()
            return redirect(url_for('status'))

    # Load current tfvars
    if os.path.exists(tfvars_path):
        data = parse_tfvars(open(tfvars_path).read())
    else:
        data = {}

    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Local Terraform Deployment - {{ template_type|upper }}</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            * { margin: 0; padding: 0; box-sizing: border-box; }
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif;
                background: #1b1b1d;
                color: #e8e8e8;
                padding: 20px;
                line-height: 1.6;
            }
            .container {
                max-width: 900px;
                margin: 0 auto;
                background: #242426;
                padding: 40px;
                border-radius: 8px;
                border: 1px solid #35353a;
            }
            h1 {
                color: #ffffff;
                margin-bottom: 10px;
                font-size: 32px;
                font-weight: 600;
            }
            .subtitle {
                color: #a6a6a6;
                margin-bottom: 40px;
                font-size: 16px;
            }
            h2 {
                color: #ffffff;
                margin-top: 30px;
                margin-bottom: 20px;
                font-size: 20px;
                font-weight: 600;
                border-bottom: 2px solid #ff6b35;
                padding-bottom: 8px;
            }
            .form-group { margin-bottom: 20px; }
            label {
                display: block;
                margin-bottom: 6px;
                color: #e8e8e8;
                font-size: 14px;
                font-weight: 500;
            }
            input[type="text"], input[type="password"], textarea {
                width: 100%;
                padding: 10px 12px;
                background: #1b1b1d;
                border: 1px solid #35353a;
                border-radius: 6px;
                color: #ffffff;
                font-size: 14px;
                transition: all 0.2s;
                font-family: inherit;
            }
            textarea { resize: vertical; min-height: 60px; }
            input[type="text"]:focus, input[type="password"]:focus, textarea:focus {
                outline: none;
                border-color: #ff6b35;
                background: #2a2a2d;
            }
            input[type="checkbox"] {
                width: 18px;
                height: 18px;
                margin-right: 8px;
                cursor: pointer;
            }
            .checkbox-label {
                display: flex;
                align-items: center;
                cursor: pointer;
                font-weight: normal;
            }
            .btn {
                background: #ff6b35;
                color: white;
                padding: 12px 24px;
                border: none;
                border-radius: 6px;
                cursor: pointer;
                font-size: 14px;
                font-weight: 600;
                margin-right: 10px;
                margin-top: 10px;
                transition: all 0.2s;
            }
            .btn:hover {
                background: #ff7d4d;
                transform: translateY(-1px);
            }
            .btn:disabled {
                background: #555;
                cursor: not-allowed;
                transform: none;
            }
            .btn-secondary {
                background: #35353a;
            }
            .btn-secondary:hover {
                background: #404045;
            }
            .btn-success { background: #4ec9b0; }
            .btn-success:hover { background: #5dd6bf; }
            .actions { text-align: center; margin-top: 30px; }
            .flash {
                padding: 12px 16px;
                border-radius: 6px;
                margin-bottom: 20px;
                font-size: 14px;
            }
            .flash.success {
                background: #1e3a1e;
                border: 1px solid #4ec9b0;
                color: #4ec9b0;
            }
            .flash.error {
                background: #3a1e1e;
                border: 1px solid #f48771;
                color: #f48771;
            }
            .help-text {
                font-size: 12px;
                color: #757575;
                margin-top: 4px;
            }
            .tags-container div { margin-bottom: 10px; }
            .tags-container input {
                width: calc(50% - 5px);
                margin-right: 10px;
                display: inline-block;
            }
            .tags-container input:last-child { margin-right: 0; }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>🚀 Local Terraform Deployment</h1>
            {% if template_type %}
            <p class="subtitle">Template: <strong style="color: #ff6b35;">{{ template_type }}</strong></p>
            {% endif %}
            <p class="subtitle">Configure your Terraform variables and deploy your infrastructure</p>

            {% with messages = get_flashed_messages(with_categories=true) %}
                {% if messages %}
                    {% for category, message in messages %}
                        <div class="flash {{ category }}">{{ message }}</div>
                    {% endfor %}
                {% endif %}
            {% endwith %}

            <form method="post">
                <h2>Configuration</h2>
                {% for key, value in data.items() %}
                <div class="form-group">
                    {% if key == 'tenant_id' %}
                    <label>{{ key }}</label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    <div class="help-text">Tenant ID is same as Directory ID from Azure Portal</div>
                    {% elif key == 'databricks_client_id' %}
                    <label>{{ key }}</label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    <div class="help-text">OAuth Client ID from Databricks Account Console</div>
                    {% elif key == 'databricks_client_secret' %}
                    <label>{{ key }}</label>
                    <input type="password" name="{{ key }}" value="{{ value }}">
                    <div class="help-text">OAuth Client Secret - keep secure!</div>
                    {% elif key == 'databricks_account_id' %}
                    <label>{{ key }}</label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    <div class="help-text">Your Databricks Account ID</div>
                    {% elif key == 'tags' %}
                    <label>{{ key }}:</label>
                    <div class="tags-container">
                        {% for k, v in value.items() %}
                        <div>
                            <input type="text" name="tags_key_{{ loop.index }}" value="{{ k }}" placeholder="Key">
                            <input type="text" name="tags_value_{{ loop.index }}" value="{{ v }}" placeholder="Value">
                        </div>
                        {% endfor %}
                    </div>
                    <button type="button" onclick="addTag()" class="btn btn-secondary" style="font-size: 13px; padding: 8px 16px;">+ Add Tag</button>
                    {% elif value is boolean %}
                    <label class="checkbox-label">
                        <input type="checkbox" name="{{ key }}" {% if value %}checked{% endif %}> {{ key }}
                    </label>
                    {% elif value is mapping %}
                    <label>{{ key }} (JSON format):</label>
                    <textarea name="{{ key }}" rows="4">{{ value | tojson }}</textarea>
                    {% else %}
                    <label>{{ key }}:</label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    {% endif %}
                </div>
                {% endfor %}

                <button type="submit" name="update" class="btn btn-success">💾 Update Configuration</button>
            </form>

            <div class="actions">
                <h2>Deployment Actions</h2>
                <form method="post" style="display: inline;">
                    <button type="submit" name="init" class="btn" style="font-size: 18px; padding: 15px 30px;">Initialize</button>
                </form>
            </div>
        </div>
        <script>
            function addTag() {
                const container = document.querySelector('.tags-container');
                const index = container.children.length + 1;
                const div = document.createElement('div');
                div.innerHTML = '<input type="text" name="tags_key_' + index + '" placeholder="Key"><input type="text" name="tags_value_' + index + '" placeholder="Value">';
                container.appendChild(div);
            }
            function copyLogs() {
                const pre = document.querySelector('pre');
                navigator.clipboard.writeText(pre.textContent).then(() => {
                    alert('Logs copied to clipboard!');
                });
            }
        </script>
    </body>
    </html>
    ''', data=data, template_type=DETECTED_TEMPLATE or 'Unknown')

@app.route('/plan', methods=['GET', 'POST'])
def plan():
    if request.method == 'POST':
        if 'plan' in request.form:
            threading.Thread(target=run_terraform_command, args=(['terraform', 'plan'], TEMPLATE_PATH)).start()
            return redirect(url_for('status'))

    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Plan Deployment</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            * { margin: 0; padding: 0; box-sizing: border-box; }
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif;
                background: #1b1b1d;
                color: #e8e8e8;
                padding: 20px;
                line-height: 1.6;
            }
            .container {
                max-width: 900px;
                margin: 0 auto;
                background: #242426;
                padding: 40px;
                border-radius: 8px;
                border: 1px solid #35353a;
                position: relative;
            }
            h1 {
                color: #ffffff;
                margin-bottom: 10px;
                font-size: 32px;
                font-weight: 600;
            }
            .subtitle {
                color: #a6a6a6;
                margin-bottom: 40px;
                font-size: 16px;
            }
            h3 {
                color: #ffffff;
                margin-bottom: 15px;
                font-size: 18px;
                font-weight: 600;
            }
            .btn {
                background: #ff6b35;
                color: white;
                padding: 12px 24px;
                border: none;
                border-radius: 6px;
                cursor: pointer;
                font-size: 14px;
                font-weight: 600;
                margin-right: 10px;
                margin-top: 10px;
                transition: all 0.2s;
                text-decoration: none;
                display: inline-block;
            }
            .btn:hover {
                background: #ff7d4d;
                transform: translateY(-1px);
            }
            .btn-secondary {
                background: #35353a;
            }
            .btn-secondary:hover {
                background: #404045;
            }
            .btn-success { background: #4ec9b0; }
            .btn-success:hover { background: #5dd6bf; }
            .actions { text-align: center; margin-top: 30px; }
            .logs {
                background: #1b1b1d;
                border: 1px solid #35353a;
                border-radius: 6px;
                padding: 16px;
                margin-top: 20px;
                max-height: 400px;
                overflow-y: auto;
            }
            pre {
                color: #e8e8e8;
                font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
                font-size: 13px;
                line-height: 1.5;
                white-space: pre-wrap;
            }
            .back-btn { position: absolute; top: 20px; left: 20px; }
        </style>
    </head>
    <body>
        <div class="container">
            <a href="/" class="btn btn-secondary back-btn">← Back</a>
            <h1>📋 Plan Deployment</h1>
            <p class="subtitle">The plan command shows what changes Terraform will make to your infrastructure without actually applying them. This is a safe way to preview the deployment.</p>
            <div class="logs">
                <h3>Initialization Logs</h3>
                <pre>{{ last_output }}</pre>
            </div>
            <div class="actions">
                <form method="post">
                    <button type="submit" name="plan" class="btn btn-success" style="font-size: 18px; padding: 15px 30px;">Run Plan</button>
                </form>
            </div>
        </div>
    </body>
    </html>
    ''', last_output=last_output)

@app.route('/apply', methods=['GET', 'POST'])
def apply():
    if request.method == 'POST':
        if 'apply' in request.form:
            threading.Thread(target=run_terraform_command, args=(['terraform', 'apply', '-auto-approve'], TEMPLATE_PATH)).start()
            return redirect(url_for('status'))

    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Apply Deployment</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            * { margin: 0; padding: 0; box-sizing: border-box; }
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif;
                background: #1b1b1d;
                color: #e8e8e8;
                padding: 20px;
                line-height: 1.6;
            }
            .container {
                max-width: 900px;
                margin: 0 auto;
                background: #242426;
                padding: 40px;
                border-radius: 8px;
                border: 1px solid #35353a;
                position: relative;
            }
            h1 {
                color: #ffffff;
                margin-bottom: 10px;
                font-size: 32px;
                font-weight: 600;
            }
            .subtitle {
                color: #a6a6a6;
                margin-bottom: 40px;
                font-size: 16px;
            }
            h3 {
                color: #ffffff;
                margin-bottom: 15px;
                font-size: 18px;
                font-weight: 600;
            }
            .btn {
                background: #ff6b35;
                color: white;
                padding: 12px 24px;
                border: none;
                border-radius: 6px;
                cursor: pointer;
                font-size: 14px;
                font-weight: 600;
                margin-right: 10px;
                margin-top: 10px;
                transition: all 0.2s;
                text-decoration: none;
                display: inline-block;
            }
            .btn:hover {
                background: #ff7d4d;
                transform: translateY(-1px);
            }
            .btn-secondary {
                background: #35353a;
            }
            .btn-secondary:hover {
                background: #404045;
            }
            .btn-success { background: #4ec9b0; }
            .btn-success:hover { background: #5dd6bf; }
            .actions { text-align: center; margin-top: 30px; }
            .logs {
                background: #1b1b1d;
                border: 1px solid #35353a;
                border-radius: 6px;
                padding: 16px;
                margin-top: 20px;
                max-height: 400px;
                overflow-y: auto;
            }
            pre {
                color: #e8e8e8;
                font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
                font-size: 13px;
                line-height: 1.5;
                white-space: pre-wrap;
            }
            .back-btn { position: absolute; top: 20px; left: 20px; }
        </style>
    </head>
    <body>
        <div class="container">
            <a href="/" class="btn btn-secondary back-btn">← Back</a>
            <h1>🚀 Apply Deployment</h1>
            <p class="subtitle">The apply command will create or update your infrastructure according to the plan. This will make actual changes to your cloud resources.</p>
            <div class="logs">
                <h3>Planning Logs</h3>
                <pre>{{ last_output }}</pre>
            </div>
            <div class="actions">
                <form method="post">
                    <button type="submit" name="apply" class="btn btn-success" style="font-size: 18px; padding: 15px 30px;">✓ Apply Changes</button>
                </form>
            </div>
        </div>
    </body>
    </html>
    ''', last_output=last_output)

@app.route('/status')
def status():
    global deploy_status
    if deploy_status['running']:
        return render_template_string('''
        <!DOCTYPE html>
        <html>
        <head>
            <title>Deployment Status</title>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <style>
                * { margin: 0; padding: 0; box-sizing: border-box; }
                body {
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif;
                    background: #1b1b1d;
                    color: #e8e8e8;
                    padding: 20px;
                    line-height: 1.6;
                }
                .container {
                    max-width: 900px;
                    margin: 0 auto;
                    background: #242426;
                    padding: 40px;
                    border-radius: 8px;
                    border: 1px solid #35353a;
                    position: relative;
                }
                h1 {
                    color: #ffffff;
                    margin-bottom: 10px;
                    font-size: 32px;
                    font-weight: 600;
                }
                .subtitle {
                    color: #a6a6a6;
                    margin-bottom: 40px;
                    font-size: 16px;
                }
                pre {
                    background: #1b1b1d;
                    border: 1px solid #35353a;
                    border-radius: 6px;
                    padding: 16px;
                    max-height: 500px;
                    overflow-y: auto;
                    color: #e8e8e8;
                    font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
                    font-size: 13px;
                    line-height: 1.5;
                    white-space: pre-wrap;
                }
                .btn {
                    background: #ff6b35;
                    color: white;
                    padding: 12px 24px;
                    border: none;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 14px;
                    font-weight: 600;
                    text-decoration: none;
                    display: inline-block;
                    margin-top: 20px;
                    transition: all 0.2s;
                }
                .btn:hover {
                    background: #ff7d4d;
                    transform: translateY(-1px);
                }
                .btn-secondary {
                    background: #35353a;
                }
                .btn-secondary:hover {
                    background: #404045;
                }
                .loading {
                    color: #ff6b35;
                    font-style: italic;
                }
                .spinner {
                    display: inline-block;
                    width: 14px;
                    height: 14px;
                    border: 2px solid #555;
                    border-top-color: #ff6b35;
                    border-radius: 50%;
                    animation: spin 0.8s linear infinite;
                    margin-right: 8px;
                    vertical-align: middle;
                }
                @keyframes spin {
                    to { transform: rotate(360deg); }
                }
                .back-btn { position: absolute; top: 20px; left: 20px; }
            </style>
            <script>
                window.onload = function() {
                    var logContainer = document.querySelector('pre');
                    if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                    }
                }
                function copyLogs() {
                    const pre = document.querySelector('pre');
                    navigator.clipboard.writeText(pre.textContent).then(() => {
                        alert('Logs copied to clipboard!');
                    });
                }
            </script>
        </head>
        <body>
            <div class="container">
                <a href="/" class="btn btn-secondary back-btn">← Back</a>
                <h1>⚙️ Deployment Status</h1>
                <p class="subtitle">
                    <span class="spinner"></span>
                    <span class="loading">Running: {{ command }}...</span>
                </p>
                <pre>{{ output }}</pre>
                <meta http-equiv="refresh" content="3">
            </div>
        </body>
        </html>
        ''', output=deploy_status['output'], command=deploy_status['command'])
    else:
        return render_template_string('''
        <!DOCTYPE html>
        <html>
        <head>
            <title>Deployment Result</title>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <style>
                * { margin: 0; padding: 0; box-sizing: border-box; }
                body {
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif;
                    background: #1b1b1d;
                    color: #e8e8e8;
                    padding: 20px;
                    line-height: 1.6;
                }
                .container {
                    max-width: 900px;
                    margin: 0 auto;
                    background: #242426;
                    padding: 40px;
                    border-radius: 8px;
                    border: 1px solid #35353a;
                    position: relative;
                }
                h1 {
                    color: #ffffff;
                    margin-bottom: 10px;
                    font-size: 32px;
                    font-weight: 600;
                }
                pre {
                    background: #1b1b1d;
                    border: 1px solid #35353a;
                    border-radius: 6px;
                    padding: 16px;
                    max-height: 500px;
                    overflow-y: auto;
                    color: #e8e8e8;
                    font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
                    font-size: 13px;
                    line-height: 1.5;
                    white-space: pre-wrap;
                }
                .btn {
                    background: #ff6b35;
                    color: white;
                    padding: 12px 24px;
                    border: none;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 14px;
                    font-weight: 600;
                    text-decoration: none;
                    display: inline-block;
                    margin-top: 20px;
                    transition: all 0.2s;
                }
                .btn:hover {
                    background: #ff7d4d;
                    transform: translateY(-1px);
                }
                .btn-secondary {
                    background: #35353a;
                }
                .btn-secondary:hover {
                    background: #404045;
                }
                .success {
                    color: #4ec9b0;
                    font-weight: bold;
                    font-size: 18px;
                    margin-bottom: 20px;
                }
                .error {
                    color: #f48771;
                    font-weight: bold;
                    font-size: 18px;
                    margin-bottom: 20px;
                }
                .back-btn { position: absolute; top: 20px; left: 20px; }
            </style>
            <script>
                window.onload = function() {
                    var logContainer = document.querySelector('pre');
                    if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                    }
                }
                function copyLogs() {
                    const pre = document.querySelector('pre');
                    navigator.clipboard.writeText(pre.textContent).then(() => {
                        alert('Logs copied to clipboard!');
                    });
                }
            </script>
        </head>
        <body>
            <div class="container">
                <a href="/" class="btn btn-secondary back-btn">← Back</a>
                <h1>{% if success %}✓ Deployment Result{% else %}✗ Deployment Failed{% endif %}</h1>
                
                {% if success %}
                <p class="success">✓ Command completed successfully!</p>
                {% else %}
                <p class="error">✗ Command failed! Check the logs below for details.</p>
                {% endif %}
                
                <pre>{{ output }}</pre>
                
                <div style="margin-top: 20px;">
                    <button onclick="copyLogs()" class="btn btn-secondary">📋 Copy Logs</button>
                    {% if command.startswith('terraform init') and success %}
                    <a href="{{ url_for('plan') }}" class="btn">Next: Plan →</a>
                    {% elif command.startswith('terraform plan') and success %}
                    <a href="{{ url_for('apply') }}" class="btn">Next: Apply →</a>
                    {% endif %}
                </div>
            </div>
        </body>
        </html>
        ''', output=deploy_status['output'], success=deploy_status['success'], command=deploy_status['command'])

if __name__ == '__main__':
    if not TEMPLATE_PATH:
        print("ERROR: Could not find terraform.tfvars file.")
        print("Please ensure you're running this script from the template directory or the workspace directory.")
        print("\nAvailable templates:")
        for name, path in TEMPLATE_DIRS.items():
            exists = "✓" if os.path.exists(path) else "✗"
            print(f"  {exists} {name}: {path}")
        print("\nUsage: python3 deploy.py [azure-simple|aws-simple]")
        sys.exit(1)
    
    print(f"Detected template: {DETECTED_TEMPLATE}")
    print(f"Template path: {TEMPLATE_PATH}")
    print(f"Starting deployment UI on http://127.0.0.1:8081")
    print("\nTo switch templates, run:")
    print("  python3 deploy.py azure-simple")
    print("  python3 deploy.py aws-simple")
    
    if HAS_WEBVIEW:
        # Start Flask server in a thread
        threading.Thread(target=lambda: app.run(debug=False, host='127.0.0.1', port=8081)).start()
        
        # Open desktop window
        webview.create_window(f'Terraform Deployment - {DETECTED_TEMPLATE}', 'http://127.0.0.1:8081')
        webview.start()
    else:
        # Run Flask directly (webview not available)
        print("Note: pywebview not installed. Opening in browser instead.")
        print("Install with: pip install pywebview")
        import webbrowser
        threading.Timer(1.5, lambda: webbrowser.open('http://127.0.0.1:8081')).start()
        app.run(debug=False, host='127.0.0.1', port=8081)