#!/usr/bin/env python3
"""
AWS Databricks Workspace Deployment UI

A local web interface for configuring and deploying AWS Databricks workspaces.
This tool helps you populate terraform.tfvars and run Terraform commands.
"""

import os
import sys
import subprocess
import threading
import webbrowser
from flask import Flask, render_template_string, request, redirect, url_for, flash, jsonify

app = Flask(__name__)
app.secret_key = 'aws-deployment-key'

# Get the directory where this script is located
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
TFVARS_PATH = os.path.join(SCRIPT_DIR, 'terraform.tfvars')

# Global variable to store deployment status
deploy_status = {'running': False, 'output': '', 'success': None, 'command': None}

def parse_tfvars(filepath):
    """Parse terraform.tfvars file into a dictionary"""
    data = {}
    if not os.path.exists(filepath):
        return data
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    lines = content.split('\n')
    for line in lines:
        line = line.strip()
        if '=' in line and not line.startswith('#'):
            key, value = line.split('=', 1)
            key = key.strip()
            value = value.strip()
            
            # Remove quotes
            if value.startswith('"') and value.endswith('"'):
                data[key] = value[1:-1]
            # Handle lists
            elif value.startswith('[') and value.endswith(']'):
                # Simple list parsing
                items = value[1:-1].split(',')
                data[key] = [item.strip().strip('"') for item in items if item.strip()]
            # Handle booleans
            elif value.lower() in ['true', 'false']:
                data[key] = value.lower() == 'true'
            else:
                data[key] = value.strip('"')
    
    return data

def save_tfvars(data, filepath):
    """Save configuration data to terraform.tfvars"""
    lines = []
    lines.append("# " + "=" * 77)
    lines.append("# Databricks Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'databricks_account_id = "{data.get("databricks_account_id", "")}"')
    lines.append(f'databricks_client_id = "{data.get("databricks_client_id", "")}"')
    lines.append(f'databricks_client_secret = "{data.get("databricks_client_secret", "")}"')
    lines.append(f'prefix = "{data.get("prefix", "databricks-workspace")}"')
    lines.append(f'resource_prefix = "{data.get("resource_prefix", "databricks-workspace")}"')
    lines.append(f'pricing_tier = "{data.get("pricing_tier", "PREMIUM")}"')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# AWS Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'region = "{data.get("region", "us-west-2")}"')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Network Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'vpc_id = "{data.get("vpc_id", "")}"')
    lines.append(f'vpc_cidr_range = "{data.get("vpc_cidr_range", "10.0.0.0/16")}"')
    
    # Handle availability zones (list)
    azs = data.get("availability_zones", ["us-west-2a", "us-west-2b"])
    if isinstance(azs, str):
        azs = [azs]
    lines.append(f'availability_zones = {json.dumps(azs)}')
    
    # Handle subnet CIDRs (lists)
    private_subnets = data.get("private_subnets_cidr", ["10.0.1.0/24", "10.0.2.0/24"])
    if isinstance(private_subnets, str):
        private_subnets = [private_subnets]
    lines.append(f'private_subnets_cidr = {json.dumps(private_subnets)}')
    
    public_subnets = data.get("public_subnets_cidr", ["10.0.101.0/24", "10.0.102.0/24"])
    if isinstance(public_subnets, str):
        public_subnets = [public_subnets]
    lines.append(f'public_subnets_cidr = {json.dumps(public_subnets)}')
    
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Security Group Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append('security_group_ids = []')
    lines.append('sg_egress_ports = [443, 3306, 2443, 8443, 8444, 8445, 8446, 8447, 8448, 8449, 8450, 8451]')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Unity Catalog Metastore Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'metastore_id = "{data.get("metastore_id", "")}"')
    lines.append(f'metastore_name = "{data.get("metastore_name", "")}"')
    lines.append("")
    
    with open(filepath, 'w') as f:
        f.write('\n'.join(lines))

import json

def run_terraform_command(command):
    """Run a terraform command in a background thread"""
    global deploy_status
    
    def run():
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
                cwd=SCRIPT_DIR
            )
            
            for line in iter(process.stdout.readline, ''):
                if line:
                    deploy_status['output'] += line
            
            process.wait()
            deploy_status['success'] = (process.returncode == 0)
        except Exception as e:
            deploy_status['output'] += f'\n\nError: {str(e)}'
            deploy_status['success'] = False
        finally:
            deploy_status['running'] = False
    
    thread = threading.Thread(target=run, daemon=True)
    thread.start()

# HTML Template
HTML_TEMPLATE = '''
<!DOCTYPE html>
<html>
<head>
    <title>AWS Databricks Deployment</title>
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
        .form-group {
            margin-bottom: 20px;
        }
        label {
            display: block;
            margin-bottom: 6px;
            color: #e8e8e8;
            font-size: 14px;
            font-weight: 500;
        }
        input[type="text"], input[type="password"], select {
            width: 100%;
            padding: 10px 12px;
            background: #1b1b1d;
            border: 1px solid #35353a;
            border-radius: 6px;
            color: #ffffff;
            font-size: 14px;
            transition: all 0.2s;
        }
        input[type="text"]:focus, input[type="password"]:focus, select:focus {
            outline: none;
            border-color: #ff6b35;
            background: #2a2a2d;
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
        .alert {
            padding: 12px 16px;
            border-radius: 6px;
            margin-bottom: 20px;
            font-size: 14px;
        }
        .alert-success {
            background: #1e3a1e;
            border: 1px solid #4ec9b0;
            color: #4ec9b0;
        }
        .alert-error {
            background: #3a1e1e;
            border: 1px solid #f48771;
            color: #f48771;
        }
        .output {
            background: #1b1b1d;
            border: 1px solid #35353a;
            border-radius: 6px;
            padding: 16px;
            margin-top: 20px;
            max-height: 400px;
            overflow-y: auto;
            font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
            font-size: 13px;
            line-height: 1.5;
            white-space: pre-wrap;
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
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        .help-text {
            font-size: 12px;
            color: #757575;
            margin-top: 4px;
        }
        .two-column {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }
        @media (max-width: 768px) {
            .two-column { grid-template-columns: 1fr; }
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 AWS Databricks Deployment</h1>
        <p class="subtitle">Configure your AWS Databricks workspace and deploy with Terraform</p>
        
        {% with messages = get_flashed_messages(with_categories=true) %}
            {% if messages %}
                {% for category, message in messages %}
                    <div class="alert alert-{{ category }}">{{ message }}</div>
                {% endfor %}
            {% endif %}
        {% endwith %}
        
        <form method="POST" action="/save">
            <h2>Databricks Configuration</h2>
            <div class="form-group">
                <label for="databricks_account_id">Databricks Account ID *</label>
                <input type="text" id="databricks_account_id" name="databricks_account_id" 
                       value="{{ config.get('databricks_account_id', '') }}" required>
                <div class="help-text">Found in your Databricks Account Console</div>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="databricks_client_id">Service Principal Client ID *</label>
                    <input type="text" id="databricks_client_id" name="databricks_client_id" 
                           value="{{ config.get('databricks_client_id', '') }}" required>
                </div>
                <div class="form-group">
                    <label for="databricks_client_secret">Service Principal Client Secret *</label>
                    <input type="password" id="databricks_client_secret" name="databricks_client_secret" 
                           value="{{ config.get('databricks_client_secret', '') }}" required>
                </div>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="prefix">Databricks Resource Prefix *</label>
                    <input type="text" id="prefix" name="prefix" 
                           value="{{ config.get('prefix', 'databricks-workspace') }}" required>
                    <div class="help-text">Used for workspace name</div>
                </div>
                <div class="form-group">
                    <label for="pricing_tier">Pricing Tier *</label>
                    <select id="pricing_tier" name="pricing_tier">
                        <option value="PREMIUM" {% if config.get('pricing_tier') == 'PREMIUM' %}selected{% endif %}>PREMIUM</option>
                        <option value="ENTERPRISE" {% if config.get('pricing_tier') == 'ENTERPRISE' %}selected{% endif %}>ENTERPRISE</option>
                    </select>
                </div>
            </div>
            
            <h2>AWS Configuration</h2>
            <div class="two-column">
                <div class="form-group">
                    <label for="region">AWS Region *</label>
                    <select id="region" name="region">
                        {% for r in ['us-east-1', 'us-east-2', 'us-west-2', 'eu-west-1', 'eu-central-1', 'ap-southeast-1', 'ap-southeast-2', 'ap-northeast-1'] %}
                        <option value="{{ r }}" {% if config.get('region') == r %}selected{% endif %}>{{ r }}</option>
                        {% endfor %}
                    </select>
                </div>
                <div class="form-group">
                    <label for="resource_prefix">AWS Resource Prefix *</label>
                    <input type="text" id="resource_prefix" name="resource_prefix" 
                           value="{{ config.get('resource_prefix', 'databricks-workspace') }}" required>
                    <div class="help-text">Used for VPC, S3, IAM resources</div>
                </div>
            </div>
            
            <h2>Network Configuration</h2>
            <div class="form-group">
                <label for="vpc_cidr_range">VPC CIDR Range *</label>
                <input type="text" id="vpc_cidr_range" name="vpc_cidr_range" 
                       value="{{ config.get('vpc_cidr_range', '10.0.0.0/16') }}" required>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="availability_zones">Availability Zones (comma-separated) *</label>
                    <input type="text" id="availability_zones" name="availability_zones" 
                           value="{{ ','.join(config.get('availability_zones', [])) if config.get('availability_zones') else 'us-west-2a,us-west-2b' }}" required>
                </div>
                <div class="form-group">
                    <label for="private_subnets_cidr">Private Subnet CIDRs (comma-separated) *</label>
                    <input type="text" id="private_subnets_cidr" name="private_subnets_cidr" 
                           value="{{ ','.join(config.get('private_subnets_cidr', [])) if config.get('private_subnets_cidr') else '10.0.1.0/24,10.0.2.0/24' }}" required>
                </div>
            </div>
            
            <div class="form-group">
                <label for="public_subnets_cidr">Public Subnet CIDRs (comma-separated) *</label>
                <input type="text" id="public_subnets_cidr" name="public_subnets_cidr" 
                       value="{{ ','.join(config.get('public_subnets_cidr', [])) if config.get('public_subnets_cidr') else '10.0.101.0/24,10.0.102.0/24' }}" required>
            </div>
            
            <h2>Unity Catalog</h2>
            <div class="form-group">
                <label for="metastore_name">Metastore Name *</label>
                <input type="text" id="metastore_name" name="metastore_name" 
                       value="{{ config.get('metastore_name', '') }}" required>
                <div class="help-text">Leave empty to use existing metastore</div>
            </div>
            
            <button type="submit" class="btn">💾 Save Configuration</button>
        </form>
        
        <h2>Terraform Commands</h2>
        <div>
            <button onclick="runCommand('init')" class="btn" id="btn-init" 
                    {% if status.running %}disabled{% endif %}>Initialize Terraform</button>
            <button onclick="runCommand('plan')" class="btn btn-secondary" id="btn-plan" 
                    {% if status.running %}disabled{% endif %}>Plan Deployment</button>
            <button onclick="runCommand('apply')" class="btn" id="btn-apply" 
                    {% if status.running %}disabled{% endif %}>Deploy Workspace</button>
            <button onclick="runCommand('destroy')" class="btn btn-secondary" id="btn-destroy" 
                    {% if status.running %}disabled{% endif %}>Destroy Workspace</button>
        </div>
        
        {% if status.command %}
        <div class="output" id="output">
            <strong>Command:</strong> {{ status.command }}<br><br>
            <div id="output-text">{{ status.output }}</div>
            {% if status.running %}
            <div style="margin-top: 10px;"><span class="spinner"></span>Running...</div>
            {% elif status.success == True %}
            <div style="margin-top: 10px; color: #4ec9b0;">✓ Success</div>
            {% elif status.success == False %}
            <div style="margin-top: 10px; color: #f48771;">✗ Failed</div>
            {% endif %}
        </div>
        {% endif %}
    </div>
    
    <script>
        function runCommand(cmd) {
            fetch('/run/' + cmd, {method: 'POST'})
                .then(() => {
                    setTimeout(checkStatus, 500);
                });
        }
        
        function checkStatus() {
            fetch('/status')
                .then(r => r.json())
                .then(data => {
                    if (data.running || data.command) {
                        location.reload();
                    }
                });
        }
        
        {% if status.running %}
        setInterval(checkStatus, 2000);
        {% endif %}
    </script>
</body>
</html>
'''

@app.route('/')
def index():
    config = parse_tfvars(TFVARS_PATH)
    return render_template_string(HTML_TEMPLATE, config=config, status=deploy_status)

@app.route('/save', methods=['POST'])
def save():
    data = {}
    
    # Get all form fields
    data['databricks_account_id'] = request.form.get('databricks_account_id', '')
    data['databricks_client_id'] = request.form.get('databricks_client_id', '')
    data['databricks_client_secret'] = request.form.get('databricks_client_secret', '')
    data['prefix'] = request.form.get('prefix', 'databricks-workspace')
    data['resource_prefix'] = request.form.get('resource_prefix', 'databricks-workspace')
    data['pricing_tier'] = request.form.get('pricing_tier', 'PREMIUM')
    data['region'] = request.form.get('region', 'us-west-2')
    data['vpc_cidr_range'] = request.form.get('vpc_cidr_range', '10.0.0.0/16')
    data['vpc_id'] = ''
    data['metastore_id'] = ''
    data['metastore_name'] = request.form.get('metastore_name', '')
    
    # Handle lists
    data['availability_zones'] = [az.strip() for az in request.form.get('availability_zones', '').split(',') if az.strip()]
    data['private_subnets_cidr'] = [s.strip() for s in request.form.get('private_subnets_cidr', '').split(',') if s.strip()]
    data['public_subnets_cidr'] = [s.strip() for s in request.form.get('public_subnets_cidr', '').split(',') if s.strip()]
    
    save_tfvars(data, TFVARS_PATH)
    flash('Configuration saved successfully!', 'success')
    return redirect(url_for('index'))

@app.route('/run/<command>', methods=['POST'])
def run_command(command):
    if deploy_status['running']:
        return jsonify({'error': 'A command is already running'}), 400
    
    cmd_map = {
        'init': ['terraform', 'init'],
        'plan': ['terraform', 'plan'],
        'apply': ['terraform', 'apply', '-auto-approve'],
        'destroy': ['terraform', 'destroy', '-auto-approve']
    }
    
    if command not in cmd_map:
        return jsonify({'error': 'Invalid command'}), 400
    
    run_terraform_command(cmd_map[command])
    return jsonify({'success': True})

@app.route('/status')
def status():
    return jsonify(deploy_status)

if __name__ == '__main__':
    print("\n" + "="*60)
    print("  AWS Databricks Workspace Deployment Tool")
    print("="*60)
    print(f"\n  Opening web interface at: http://localhost:8080")
    print(f"  Template directory: {SCRIPT_DIR}")
    print(f"\n  Press Ctrl+C to stop the server")
    print("="*60 + "\n")
    
    # Open browser
    threading.Timer(1.5, lambda: webbrowser.open('http://localhost:8080')).start()
    
    # Run Flask app
    app.run(host='127.0.0.1', port=8080, debug=False)
