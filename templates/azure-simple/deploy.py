#!/usr/bin/env python3
"""
Azure Databricks Workspace Deployment UI

A local web interface for configuring and deploying Azure Databricks workspaces.
This tool helps you populate terraform.tfvars and run Terraform commands.
"""

import os
import sys
import subprocess
import threading
import webbrowser
import json
from flask import Flask, render_template_string, request, redirect, url_for, flash, jsonify

app = Flask(__name__)
app.secret_key = 'azure-deployment-key'

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
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        
        # Handle multi-line tags block
        if line.startswith('tags') and '=' in line:
            # Start of tags
            tag_content = line
            if '{' in line and '}' not in line:
                # Multi-line tags
                i += 1
                while i < len(lines) and '}' not in lines[i]:
                    tag_content += '\n' + lines[i]
                    i += 1
                if i < len(lines):
                    tag_content += '\n' + lines[i]
            
            # Parse tags
            data['tags'] = {}
            # Extract content between braces
            if '{' in tag_content and '}' in tag_content:
                tags_str = tag_content[tag_content.find('{')+1:tag_content.rfind('}')]
                for tag_line in tags_str.split('\n'):
                    tag_line = tag_line.strip()
                    if '=' in tag_line:
                        k, v = tag_line.split('=', 1)
                        k = k.strip().strip('"')
                        v = v.strip().strip('"').rstrip(',')
                        if k:
                            data['tags'][k] = v
        elif '=' in line and not line.startswith('#'):
            key, value = line.split('=', 1)
            key = key.strip()
            value = value.strip()
            
            # Remove quotes
            if value.startswith('"') and value.endswith('"'):
                data[key] = value[1:-1]
            # Handle booleans
            elif value.lower() in ['true', 'false']:
                data[key] = value.lower() == 'true'
            else:
                data[key] = value.strip('"')
        
        i += 1
    
    return data

def save_tfvars(data, filepath):
    """Save configuration data to terraform.tfvars"""
    lines = []
    lines.append("# " + "=" * 77)
    lines.append("# Azure Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'tenant_id = "{data.get("tenant_id", "")}"')
    lines.append(f'azure_subscription_id = "{data.get("azure_subscription_id", "")}"')
    
    # Handle tags
    tags = data.get("tags", {})
    if tags:
        lines.append("tags = {")
        for k, v in tags.items():
            lines.append(f'  "{k}" = "{v}"')
        lines.append("}")
    else:
        lines.append("tags = {")
        lines.append("}")
    
    lines.append(f'resource_group_name = "{data.get("resource_group_name", "")}"')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Databricks Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'workspace_name = "{data.get("workspace_name", "")}"')
    lines.append(f'admin_user = "{data.get("admin_user", "")}"')
    lines.append(f'root_storage_name = "{data.get("root_storage_name", "")}"')
    lines.append(f'location = "{data.get("location", "northeurope")}"')
    lines.append(f'databricks_account_id = "{data.get("databricks_account_id", "")}"')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Unity Catalog Metastore Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    lines.append(f'existing_metastore_id = "{data.get("existing_metastore_id", "")}"')
    lines.append(f'new_metastore_name = "{data.get("new_metastore_name", "")}"')
    lines.append("")
    lines.append("# " + "=" * 77)
    lines.append("# Network Configuration")
    lines.append("# " + "=" * 77)
    lines.append("")
    create_vnet = data.get("create_new_vnet", True)
    lines.append(f'create_new_vnet = {str(create_vnet).lower()}')
    lines.append(f'vnet_name = "{data.get("vnet_name", "")}"')
    lines.append(f'vnet_resource_group_name = "{data.get("vnet_resource_group_name", "")}"')
    lines.append(f'cidr = "{data.get("cidr", "10.0.0.0/20")}"')
    lines.append(f'subnet_public_cidr = "{data.get("subnet_public_cidr", "10.0.1.0/24")}"')
    lines.append(f'subnet_private_cidr = "{data.get("subnet_private_cidr", "10.0.2.0/24")}"')
    
    with open(filepath, 'w') as f:
        f.write('\n'.join(lines))

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
    <title>Azure Databricks Deployment</title>
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
            border-bottom: 2px solid #0078d4;
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
        input[type="text"], input[type="password"], input[type="email"], select, textarea {
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
        textarea {
            resize: vertical;
            min-height: 60px;
        }
        input[type="text"]:focus, input[type="password"]:focus, input[type="email"]:focus, select:focus, textarea:focus {
            outline: none;
            border-color: #0078d4;
            background: #2a2a2d;
        }
        input[type="checkbox"] {
            width: 18px;
            height: 18px;
            margin-right: 8px;
            cursor: pointer;
        }
        .checkbox-group {
            display: flex;
            align-items: center;
            margin-bottom: 20px;
        }
        .checkbox-group label {
            margin-bottom: 0;
            cursor: pointer;
        }
        .btn {
            background: #0078d4;
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
            background: #106ebe;
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
            border-top-color: #0078d4;
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
        <h1>☁️ Azure Databricks Deployment</h1>
        <p class="subtitle">Configure your Azure Databricks workspace and deploy with Terraform</p>
        
        {% with messages = get_flashed_messages(with_categories=true) %}
            {% if messages %}
                {% for category, message in messages %}
                    <div class="alert alert-{{ category }}">{{ message }}</div>
                {% endfor %}
            {% endif %}
        {% endwith %}
        
        <form method="POST" action="/save">
            <h2>Azure Configuration</h2>
            <div class="two-column">
                <div class="form-group">
                    <label for="tenant_id">Azure Tenant ID *</label>
                    <input type="text" id="tenant_id" name="tenant_id" 
                           value="{{ config.get('tenant_id', '') }}" required>
                    <div class="help-text">Found in Azure Portal > Azure Active Directory</div>
                </div>
                <div class="form-group">
                    <label for="azure_subscription_id">Azure Subscription ID *</label>
                    <input type="text" id="azure_subscription_id" name="azure_subscription_id" 
                           value="{{ config.get('azure_subscription_id', '') }}" required>
                    <div class="help-text">Found in Azure Portal > Subscriptions</div>
                </div>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="resource_group_name">Resource Group Name *</label>
                    <input type="text" id="resource_group_name" name="resource_group_name" 
                           value="{{ config.get('resource_group_name', '') }}" required>
                    <div class="help-text">Azure resource group for all resources</div>
                </div>
                <div class="form-group">
                    <label for="location">Azure Region *</label>
                    <select id="location" name="location" required>
                        {% for r in ['northeurope', 'westeurope', 'eastus', 'eastus2', 'westus', 'westus2', 'westus3', 'centralus', 'southcentralus', 'northcentralus', 'westcentralus', 'canadacentral', 'canadaeast', 'brazilsouth', 'uksouth', 'ukwest', 'francecentral', 'germanywestcentral', 'norwayeast', 'switzerlandnorth', 'switzerlandwest', 'swedencentral', 'australiaeast', 'australiasoutheast', 'australiacentral', 'australiacentral2', 'eastasia', 'southeastasia', 'japaneast', 'japanwest', 'koreacentral', 'centralindia', 'southindia', 'westindia', 'southafricanorth', 'uaenorth', 'qatarcentral', 'mexicocentral'] %}
                        <option value="{{ r }}" {% if config.get('location') == r %}selected{% endif %}>{{ r }}</option>
                        {% endfor %}
                    </select>
                </div>
            </div>
            
            <div class="form-group">
                <label for="tags">Tags (optional, format: key=value, one per line)</label>
                <textarea id="tags" name="tags" placeholder="Environment=Production&#10;CostCenter=Engineering">{% if config.get('tags') %}{% for k, v in config.get('tags').items() %}{{ k }}={{ v }}
{% endfor %}{% endif %}</textarea>
                <div class="help-text">Optional tags for Azure resources</div>
            </div>
            
            <h2>Databricks Configuration</h2>
            <div class="form-group">
                <label for="databricks_account_id">Databricks Account ID *</label>
                <input type="text" id="databricks_account_id" name="databricks_account_id" 
                       value="{{ config.get('databricks_account_id', '') }}" required>
                <div class="help-text">Found in your Databricks Account Console</div>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="workspace_name">Workspace Name *</label>
                    <input type="text" id="workspace_name" name="workspace_name" 
                           value="{{ config.get('workspace_name', '') }}" required>
                </div>
                <div class="form-group">
                    <label for="admin_user">Admin User Email *</label>
                    <input type="email" id="admin_user" name="admin_user" 
                           value="{{ config.get('admin_user', '') }}" required>
                    <div class="help-text">Email for workspace admin access</div>
                </div>
            </div>
            
            <div class="form-group">
                <label for="root_storage_name">Root Storage Account Name *</label>
                <input type="text" id="root_storage_name" name="root_storage_name" 
                       value="{{ config.get('root_storage_name', '') }}" 
                       pattern="[a-z0-9]{3,24}" required>
                <div class="help-text">Only lowercase letters and numbers, 3-24 characters</div>
            </div>
            
            <h2>Unity Catalog Configuration</h2>
            <div class="two-column">
                <div class="form-group">
                    <label for="existing_metastore_id">Existing Metastore ID (optional)</label>
                    <input type="text" id="existing_metastore_id" name="existing_metastore_id" 
                           value="{{ config.get('existing_metastore_id', '') }}">
                    <div class="help-text">Leave empty to create new metastore</div>
                </div>
                <div class="form-group">
                    <label for="new_metastore_name">New Metastore Name</label>
                    <input type="text" id="new_metastore_name" name="new_metastore_name" 
                           value="{{ config.get('new_metastore_name', '') }}"
                           pattern="[a-zA-Z0-9_-]*">
                    <div class="help-text">Only if creating new metastore</div>
                </div>
            </div>
            
            <h2>Network Configuration</h2>
            <div class="checkbox-group">
                <input type="checkbox" id="create_new_vnet" name="create_new_vnet" 
                       {% if config.get('create_new_vnet', True) %}checked{% endif %}>
                <label for="create_new_vnet">Create New Virtual Network</label>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="vnet_name">VNet Name *</label>
                    <input type="text" id="vnet_name" name="vnet_name" 
                           value="{{ config.get('vnet_name', '') }}" required>
                </div>
                <div class="form-group">
                    <label for="vnet_resource_group_name">VNet Resource Group Name *</label>
                    <input type="text" id="vnet_resource_group_name" name="vnet_resource_group_name" 
                           value="{{ config.get('vnet_resource_group_name', '') }}" required>
                </div>
            </div>
            
            <div class="form-group">
                <label for="cidr">VNet CIDR Range *</label>
                <input type="text" id="cidr" name="cidr" 
                       value="{{ config.get('cidr', '10.0.0.0/20') }}" required>
            </div>
            
            <div class="two-column">
                <div class="form-group">
                    <label for="subnet_public_cidr">Public Subnet CIDR *</label>
                    <input type="text" id="subnet_public_cidr" name="subnet_public_cidr" 
                           value="{{ config.get('subnet_public_cidr', '10.0.1.0/24') }}" required>
                </div>
                <div class="form-group">
                    <label for="subnet_private_cidr">Private Subnet CIDR *</label>
                    <input type="text" id="subnet_private_cidr" name="subnet_private_cidr" 
                           value="{{ config.get('subnet_private_cidr', '10.0.2.0/24') }}" required>
                </div>
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
    data['tenant_id'] = request.form.get('tenant_id', '')
    data['azure_subscription_id'] = request.form.get('azure_subscription_id', '')
    data['resource_group_name'] = request.form.get('resource_group_name', '')
    data['location'] = request.form.get('location', 'northeurope')
    data['databricks_account_id'] = request.form.get('databricks_account_id', '')
    data['workspace_name'] = request.form.get('workspace_name', '')
    data['admin_user'] = request.form.get('admin_user', '')
    data['root_storage_name'] = request.form.get('root_storage_name', '')
    data['existing_metastore_id'] = request.form.get('existing_metastore_id', '')
    data['new_metastore_name'] = request.form.get('new_metastore_name', '')
    data['create_new_vnet'] = 'create_new_vnet' in request.form
    data['vnet_name'] = request.form.get('vnet_name', '')
    data['vnet_resource_group_name'] = request.form.get('vnet_resource_group_name', '')
    data['cidr'] = request.form.get('cidr', '10.0.0.0/20')
    data['subnet_public_cidr'] = request.form.get('subnet_public_cidr', '10.0.1.0/24')
    data['subnet_private_cidr'] = request.form.get('subnet_private_cidr', '10.0.2.0/24')
    
    # Parse tags
    tags_text = request.form.get('tags', '').strip()
    tags = {}
    if tags_text:
        for line in tags_text.split('\n'):
            line = line.strip()
            if '=' in line:
                k, v = line.split('=', 1)
                tags[k.strip()] = v.strip()
    data['tags'] = tags
    
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
    print("  Azure Databricks Workspace Deployment Tool")
    print("="*60)
    print(f"\n  Opening web interface at: http://localhost:8080")
    print(f"  Template directory: {SCRIPT_DIR}")
    print(f"\n  Press Ctrl+C to stop the server")
    print("="*60 + "\n")
    
    # Open browser
    threading.Timer(1.5, lambda: webbrowser.open('http://localhost:8080')).start()
    
    # Run Flask app
    app.run(host='127.0.0.1', port=8080, debug=False)
