from flask import Flask, render_template_string, request, redirect, url_for, send_file
import subprocess
import os
import re
import threading
import time
import json
import zipfile
import io
import json

app = Flask(__name__)

TEMPLATES_DIR = 'templates'

# Global variable to store apply status
apply_status = {'running': False, 'output': '', 'success': None, 'template': None}

# Global variable to store apply status
apply_status = {'running': False, 'output': '', 'success': None, 'template': None}

def get_available_templates():
    """Get list of available templates"""
    templates = []
    if os.path.exists(TEMPLATES_DIR):
        for item in os.listdir(TEMPLATES_DIR):
            template_path = os.path.join(TEMPLATES_DIR, item)
            if os.path.isdir(template_path) and os.path.exists(os.path.join(template_path, 'terraform.tfvars')):
                templates.append({
                    'name': item,
                    'path': template_path,
                    'description': get_template_description(item)
                })
    return templates

def get_template_description(template_name):
    """Get description for a template"""
    descriptions = {
        'Azure VNet Injection Workspace': 'Deploy a secure Databricks workspace with VNet injection for enhanced network isolation and security.'
    }
    return descriptions.get(template_name, f'Deploy {template_name} workspace')

def get_tfvars_path(template_name):
    """Get the path to terraform.tfvars for a template"""
    return os.path.join(TEMPLATES_DIR, template_name, 'terraform.tfvars')

def load_tfvars(template_name):
    data = {}
    tfvars_path = get_tfvars_path(template_name)
    with open(tfvars_path, 'r') as f:
        content = f.read()
    
    # Simple parsing for tfvars
    # Handle strings, booleans, maps
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

def save_tfvars(data, template_name):
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
    tfvars_path = get_tfvars_path(template_name)
    with open(tfvars_path, 'w') as f:
        f.write('\n'.join(content))

def run_terraform_apply_async(template_name):
    global apply_status
    apply_status['running'] = True
    apply_status['output'] = ''
    apply_status['success'] = None
    apply_status['template'] = template_name
    template_path = os.path.join(TEMPLATES_DIR, template_name)
    try:
        process = subprocess.Popen(['terraform', 'apply', '-auto-approve'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=template_path)
        while True:
            output = process.stdout.readline()
            if output == '' and process.poll() is not None:
                break
            if output:
                apply_status['output'] += output
        apply_status['success'] = process.returncode == 0
    except Exception as e:
        apply_status['output'] = str(e)
        apply_status['success'] = False
    apply_status['running'] = False

@app.route('/', methods=['GET'])
def index():
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Databricks Workspace Creator</title>
        <style>
            body { 
                font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
                background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%); 
                color: #ffffff; 
                margin: 0; 
                padding: 0; 
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
            }
            .container { 
                max-width: 1000px; 
                margin: 40px auto; 
                background: #1a1a1a; 
                padding: 60px; 
                border-radius: 16px; 
                box-shadow: 0 8px 32px rgba(0,0,0,0.4); 
                border: 1px solid #333; 
                text-align: center;
            }
            h1 { 
                color: #00b3ff; 
                margin-bottom: 24px; 
                font-weight: 700; 
                font-size: 48px; 
                text-transform: uppercase;
                letter-spacing: 2px;
            }
            .subtitle { 
                color: #cccccc; 
                font-size: 18px; 
                line-height: 1.6; 
                margin-bottom: 60px;
                font-weight: 400;
            }
            .cloud-selection { 
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                gap: 30px;
                margin-bottom: 40px;
            }
            .cloud-card { 
                background: #2a2a2a; 
                border: 1px solid #444; 
                border-radius: 12px; 
                padding: 40px 30px; 
                text-align: center;
                transition: all 0.3s ease;
                cursor: pointer;
                text-decoration: none;
                color: #ffffff;
                display: block;
                position: relative;
            }
            .cloud-card:hover { 
                border-color: #00b3ff; 
                box-shadow: 0 4px 16px rgba(0,179,255,0.2);
                transform: translateY(-2px);
            }
            .cloud-card.azure:hover { border-color: #0078D4; box-shadow: 0 4px 16px rgba(0,120,212,0.2); }
            .cloud-card.aws:hover { border-color: #FF9900; box-shadow: 0 4px 16px rgba(255,153,0,0.2); }
            .cloud-card.gcp:hover { border-color: #4285F4; box-shadow: 0 4px 16px rgba(66,133,244,0.2); }
            .cloud-icon { 
                font-size: 48px; 
                margin-bottom: 20px;
                display: block;
            }
            .cloud-name { 
                color: #00b3ff; 
                font-size: 24px; 
                font-weight: 600; 
                margin-bottom: 12px;
            }
            .cloud-description { 
                color: #cccccc; 
                font-size: 16px; 
                line-height: 1.5;
                margin-bottom: 16px;
            }
            .coming-soon { 
                position: absolute;
                top: 15px;
                right: 15px;
                background: #ff6b35;
                color: white;
                padding: 4px 8px;
                border-radius: 4px;
                font-size: 12px;
                font-weight: 600;
                text-transform: uppercase;
            }
            .status { 
                color: #888; 
                font-size: 14px;
                font-style: italic;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Welcome to Databricks Workspace Creator</h1>
            <p class="subtitle">This tool enables you to create a production-grade Databricks workspace following industry best practices in an intuitive and user-friendly way.</p>
            
            <h2 style="color: #00b3ff; margin-bottom: 30px;">Choose your cloud:</h2>
            
            <div class="cloud-selection">
                <a href="/azure" class="cloud-card azure">
                    <span class="cloud-icon">☁️</span>
                    <div class="cloud-name">Azure</div>
                    <div class="cloud-description">Deploy Databricks workspaces on Microsoft Azure with enterprise-grade security and compliance.</div>
                    <div class="status">Available Now</div>
                </a>
                
                <div class="cloud-card aws">
                    <span class="cloud-icon">☁️</span>
                    <div class="cloud-name">AWS</div>
                    <div class="cloud-description">Deploy Databricks workspaces on Amazon Web Services with scalable infrastructure.</div>
                    <div class="coming-soon">Coming Soon</div>
                    <div class="status">Yet to be released</div>
                </div>
                
                <div class="cloud-card gcp">
                    <span class="cloud-icon">☁️</span>
                    <div class="cloud-name">GCP</div>
                    <div class="cloud-description">Deploy Databricks workspaces on Google Cloud Platform with integrated services.</div>
                    <div class="coming-soon">Coming Soon</div>
                    <div class="status">Yet to be released</div>
                </div>
            </div>
        </div>
    </body>
    </html>
    ''')

@app.route('/azure', methods=['GET'])
def azure_templates():
    templates = get_available_templates()
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Azure Templates - Databricks Workspace Creator</title>
        <style>
            body { 
                font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
                background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%); 
                color: #ffffff; 
                margin: 0; 
                padding: 0; 
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
            }
            .container { 
                max-width: 900px; 
                margin: 40px auto; 
                background: #1a1a1a; 
                padding: 60px; 
                border-radius: 16px; 
                box-shadow: 0 8px 32px rgba(0,0,0,0.4); 
                border: 1px solid #333; 
                text-align: center;
            }
            h1 { 
                color: #0078D4; 
                margin-bottom: 24px; 
                font-weight: 700; 
                font-size: 42px; 
                text-transform: uppercase;
                letter-spacing: 2px;
            }
            .subtitle { 
                color: #cccccc; 
                font-size: 18px; 
                line-height: 1.6; 
                margin-bottom: 50px;
                font-weight: 400;
            }
            .templates { 
                display: grid;
                gap: 20px;
                margin-bottom: 40px;
            }
            .template-card { 
                background: #2a2a2a; 
                border: 1px solid #444; 
                border-radius: 12px; 
                padding: 30px; 
                text-align: left;
                transition: all 0.3s ease;
                cursor: pointer;
                text-decoration: none;
                color: #ffffff;
                display: block;
            }
            .template-card:hover { 
                border-color: #0078D4; 
                box-shadow: 0 4px 16px rgba(0,120,212,0.2);
                transform: translateY(-2px);
            }
            .template-title { 
                color: #0078D4; 
                font-size: 24px; 
                font-weight: 600; 
                margin-bottom: 12px;
            }
            .template-description { 
                color: #cccccc; 
                font-size: 16px; 
                line-height: 1.5;
                margin-bottom: 16px;
            }
            .template-features { 
                color: #888; 
                font-size: 14px;
            }
            .template-features ul { 
                list-style: none; 
                padding: 0; 
                margin: 0;
            }
            .template-features li { 
                padding: 4px 0;
            }
            .template-features li:before { 
                content: "✓ "; 
                color: #0078D4; 
                font-weight: bold;
            }
            .back-btn { 
                background: #444; 
                color: #cccccc; 
                padding: 8px 16px; 
                border: none; 
                border-radius: 6px; 
                cursor: pointer; 
                font-size: 14px; 
                text-decoration: none; 
                display: inline-block; 
                margin-bottom: 20px; 
                transition: all 0.2s ease; 
            }
            .back-btn:hover { 
                background: #555; 
                color: #ffffff; 
            }
        </style>
    </head>
    <body>
        <div class="container">
            <a href="/" class="back-btn">← Back to Cloud Selection</a>
            <h1>Azure Templates</h1>
            <p class="subtitle">Choose from our available Azure Databricks workspace templates.</p>
            
            <div class="templates">
                {% for template in templates %}
                <a href="/configure/{{ template.name | urlencode }}" class="template-card">
                    <div class="template-title">{{ template.name }}</div>
                    <div class="template-description">{{ template.description }}</div>
                    <div class="template-features">
                        <ul>
                            <li>Private networking with VNet injection</li>
                            <li>Network security groups and NAT gateway</li>
                            <li>Azure resource group isolation</li>
                            <li>Production-ready security configuration</li>
                        </ul>
                    </div>
                </a>
                {% endfor %}
            </div>
        </div>
    </body>
    </html>
    ''', templates=templates)

@app.route('/aws', methods=['GET'])
def aws_templates():
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>AWS Templates - Coming Soon</title>
        <style>
            body { 
                font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
                background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%); 
                color: #ffffff; 
                margin: 0; 
                padding: 0; 
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
            }
            .container { 
                max-width: 800px; 
                margin: 40px auto; 
                background: #1a1a1a; 
                padding: 60px; 
                border-radius: 16px; 
                box-shadow: 0 8px 32px rgba(0,0,0,0.4); 
                border: 1px solid #333; 
                text-align: center;
            }
            h1 { 
                color: #FF9900; 
                margin-bottom: 24px; 
                font-weight: 700; 
                font-size: 42px; 
                text-transform: uppercase;
                letter-spacing: 2px;
            }
            .coming-soon { 
                color: #ff6b35; 
                font-size: 24px; 
                font-weight: 600; 
                margin-bottom: 20px;
            }
            .description { 
                color: #cccccc; 
                font-size: 18px; 
                line-height: 1.6; 
                margin-bottom: 40px;
            }
            .back-btn { 
                background: #444; 
                color: #cccccc; 
                padding: 12px 24px; 
                border: none; 
                border-radius: 8px; 
                cursor: pointer; 
                font-size: 16px; 
                text-decoration: none; 
                display: inline-block; 
                transition: all 0.2s ease; 
            }
            .back-btn:hover { 
                background: #555; 
                color: #ffffff; 
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>AWS Templates</h1>
            <div class="coming-soon">🚧 Coming Soon</div>
            <p class="description">AWS Databricks workspace templates are currently under development. Stay tuned for updates!</p>
            <a href="/" class="back-btn">← Back to Cloud Selection</a>
        </div>
    </body>
    </html>
    ''')

@app.route('/gcp', methods=['GET'])
def gcp_templates():
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>GCP Templates - Coming Soon</title>
        <style>
            body { 
                font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
                background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%); 
                color: #ffffff; 
                margin: 0; 
                padding: 0; 
                min-height: 100vh;
                display: flex;
                align-items: center;
                justify-content: center;
            }
            .container { 
                max-width: 800px; 
                margin: 40px auto; 
                background: #1a1a1a; 
                padding: 60px; 
                border-radius: 16px; 
                box-shadow: 0 8px 32px rgba(0,0,0,0.4); 
                border: 1px solid #333; 
                text-align: center;
            }
            h1 { 
                color: #4285F4; 
                margin-bottom: 24px; 
                font-weight: 700; 
                font-size: 42px; 
                text-transform: uppercase;
                letter-spacing: 2px;
            }
            .coming-soon { 
                color: #ff6b35; 
                font-size: 24px; 
                font-weight: 600; 
                margin-bottom: 20px;
            }
            .description { 
                color: #cccccc; 
                font-size: 18px; 
                line-height: 1.6; 
                margin-bottom: 40px;
            }
            .back-btn { 
                background: #444; 
                color: #cccccc; 
                padding: 12px 24px; 
                border: none; 
                border-radius: 8px; 
                cursor: pointer; 
                font-size: 16px; 
                text-decoration: none; 
                display: inline-block; 
                transition: all 0.2s ease; 
            }
            .back-btn:hover { 
                background: #555; 
                color: #ffffff; 
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>GCP Templates</h1>
            <div class="coming-soon">🚧 Coming Soon</div>
            <p class="description">Google Cloud Platform Databricks workspace templates are currently under development. Stay tuned for updates!</p>
            <a href="/" class="back-btn">← Back to Cloud Selection</a>
        </div>
    </body>
    </html>
    ''')

@app.route('/configure/<path:template_name>', methods=['GET', 'POST'])
def configure(template_name):
    global apply_status
    if request.method == 'POST':
        # Update tfvars
        data = load_tfvars(template_name)
        for key in request.form:
            if key in data:
                if isinstance(data[key], bool):
                    data[key] = request.form.get(key) == 'on'
                elif isinstance(data[key], dict):
                    # For maps, assume JSON input
                    import json
                    try:
                        data[key] = json.loads(request.form[key])
                    except:
                        pass
                else:
                    data[key] = request.form[key]
        save_tfvars(data, template_name)
        
        # Run terraform apply in background
        threading.Thread(target=run_terraform_apply_async, args=(template_name,)).start()
        
        return redirect(url_for('status'))
    
    data = load_tfvars(template_name)
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Terraform Configuration - {{ template_name }}</title>
        <style>
            body { font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background-color: #0f0f0f; color: #ffffff; margin: 0; padding: 20px; }
            .container { max-width: 900px; margin: 0 auto; background: #1a1a1a; padding: 40px; border-radius: 12px; box-shadow: 0 4px 20px rgba(0,0,0,0.3); border: 1px solid #333; }
            h1 { color: #00b3ff; margin-bottom: 40px; text-align: center; font-weight: 600; font-size: 28px; }
            .form-group { margin-bottom: 24px; position: relative; }
            .info-icon { display: inline-block; width: 16px; height: 16px; border-radius: 50%; background-color: #00b3ff; color: white; text-align: center; font-size: 12px; font-weight: bold; line-height: 16px; margin-left: 8px; cursor: help; vertical-align: middle; position: relative; }
            .info-icon:hover { background-color: #0078d4; }
            .info-icon:hover::after { content: "Tenant ID is same as Directory ID, which can be found in Azure Portal under https://portal.azure.com/#settings/directory"; position: absolute; bottom: 100%; left: 50%; transform: translateX(-50%); background-color: #333; color: white; padding: 8px 12px; border-radius: 4px; white-space: nowrap; font-size: 12px; font-weight: normal; z-index: 1000; margin-bottom: 5px; }
            label { display: block; margin-bottom: 8px; font-weight: 500; color: #e0e0e0; font-size: 14px; }
            input[type="text"], textarea { width: 100%; padding: 12px 16px; border: 1px solid #444; border-radius: 8px; font-size: 14px; background-color: #2a2a2a; color: #ffffff; box-sizing: border-box; }
            input[type="text"]:focus, textarea:focus { outline: none; border-color: #00b3ff; box-shadow: 0 0 0 2px rgba(0,179,255,0.2); }
            input[type="checkbox"] { margin-right: 12px; accent-color: #00b3ff; }
            .checkbox-label { display: inline; font-weight: normal; cursor: pointer; }
            .btn { background: linear-gradient(135deg, #00b3ff 0%, #0078d4 100%); color: white; padding: 14px 28px; border: none; border-radius: 8px; cursor: pointer; font-size: 16px; font-weight: 600; width: 100%; transition: all 0.2s ease; }
            .btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(0,179,255,0.3); }
            .btn:active { transform: translateY(0); }
            .back-btn { background: #444; color: #cccccc; padding: 8px 16px; border: none; border-radius: 6px; cursor: pointer; font-size: 14px; text-decoration: none; display: inline-block; margin-bottom: 20px; transition: all 0.2s ease; }
            .back-btn:hover { background: #555; color: #ffffff; }
        </style>
    </head>
    <body>
        <div class="container">
            <a href="/" class="back-btn">← Back to Templates</a>
            <h1>Terraform Variables Configuration - {{ template_name }}</h1>
            <form method="post">
                {% for key, value in data.items() %}
                <div class="form-group">
                    {% if key == 'tenant_id' %}
                    <label>{{ key }} <span class="info-icon">i</span></label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    {% elif value is boolean %}
                    <label class="checkbox-label"><input type="checkbox" name="{{ key }}" {% if value %}checked{% endif %}> {{ key }}</label>
                    {% elif value is mapping %}
                    <label>{{ key }} (JSON):</label>
                    <textarea name="{{ key }}" rows="4">{{ value | tojson }}</textarea>
                    {% else %}
                    <label>{{ key }}:</label>
                    <input type="text" name="{{ key }}" value="{{ value }}">
                    {% endif %}
                </div>
                {% endfor %}
                <button type="submit" class="btn">Save and Apply</button>
            </form>
        </div>
    </body>
    </html>
    ''', template_name=template_name, data=data)

@app.route('/status')
def status():
    global apply_status
    if apply_status['running']:
        return render_template_string('''
        <!DOCTYPE html>
        <html>
        <head>
            <title>Terraform Apply Status</title>
            <style>
                body { font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background-color: #0f0f0f; color: #ffffff; margin: 0; padding: 20px; }
                .container { max-width: 900px; margin: 0 auto; background: #1a1a1a; padding: 40px; border-radius: 12px; box-shadow: 0 4px 20px rgba(0,0,0,0.3); border: 1px solid #333; }
                h1 { color: #00b3ff; margin-bottom: 30px; text-align: center; font-weight: 600; font-size: 28px; }
                .status { margin-top: 20px; padding: 20px; background: #2a2a2a; border-left: 4px solid #00b3ff; border-radius: 8px; }
                pre { background: #0f0f0f; padding: 20px; border-radius: 8px; overflow-x: auto; white-space: pre-wrap; max-height: 500px; overflow-y: auto; border: 1px solid #444; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 13px; line-height: 1.4; }
                .btn { background: linear-gradient(135deg, #00b3ff 0%, #0078d4 100%); color: white; padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; text-decoration: none; display: inline-block; margin-top: 20px; font-weight: 600; transition: all 0.2s ease; }
                .btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(0,179,255,0.3); }
                .loading { color: #00b3ff; font-style: italic; }
            </style>
            <script>
                window.onload = function() {
                    var logContainer = document.querySelector('pre');
                    if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                    }
                }
            </script>
        </head>
        <body>
            <div class="container">
                <h1>Terraform Apply Running...</h1>
                <p class="loading">Please wait, this may take several minutes.</p>
                <div class="status">
                    <pre>{{ output }}</pre>
                </div>
                <a href="/" class="btn">Back to Form</a>
                <meta http-equiv="refresh" content="5">
            </div>
        </body>
        </html>
        ''', output=apply_status['output'])
    else:
        return render_template_string('''
        <!DOCTYPE html>
        <html>
        <head>
            <title>Terraform Apply Result</title>
            <style>
                body { font-family: 'Inter', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background-color: #0f0f0f; color: #ffffff; margin: 0; padding: 20px; }
                .container { max-width: 900px; margin: 0 auto; background: #1a1a1a; padding: 40px; border-radius: 12px; box-shadow: 0 4px 20px rgba(0,0,0,0.3); border: 1px solid #333; }
                h1 { color: #00b3ff; margin-bottom: 30px; text-align: center; font-weight: 600; font-size: 28px; }
                .status { margin-top: 20px; padding: 20px; background: #2a2a2a; border-left: 4px solid #00b3ff; border-radius: 8px; }
                pre { background: #0f0f0f; padding: 20px; border-radius: 8px; overflow-x: auto; white-space: pre-wrap; max-height: 500px; overflow-y: auto; border: 1px solid #444; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 13px; line-height: 1.4; }
                .success { color: #00ff88; font-weight: 600; }
                .error { color: #ff6b6b; font-weight: 600; }
                .btn { background: linear-gradient(135deg, #00b3ff 0%, #0078d4 100%); color: white; padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; text-decoration: none; display: inline-block; margin-top: 20px; font-weight: 600; transition: all 0.2s ease; }
                .btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(0,179,255,0.3); }
                .download-section { margin-top: 30px; padding: 20px; background: #1a1a1a; border-radius: 8px; border: 1px solid #333; }
                .download-btn { background: linear-gradient(135deg, #28a745 0%, #20c997 100%); margin-bottom: 15px; }
                .download-btn:hover { box-shadow: 0 4px 12px rgba(40,167,69,0.3); }
                .download-note { color: #cccccc; font-size: 14px; margin: 0; line-height: 1.5; }
            </style>
            <script>
                window.onload = function() {
                    var logContainer = document.querySelector('pre');
                    if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                    }
                }
            </script>
        </head>
        <body>
            <div class="container">
                <h1>Terraform Apply Result</h1>
                <div class="status">
                    <pre>{{ output }}</pre>
                    {% if success %}
                    <p class="success">✓ Apply successful!</p>
                    <div class="download-section">
                        <a href="/download/{{ template_name | urlencode }}" class="btn download-btn">📥 Download Terraform Code</a>
                        <p class="download-note">💡 <strong>Tip:</strong> We suggest uploading this Terraform code to your Git repository and continuing to manage your workspace with it!</p>
                    </div>
                    {% else %}
                    <p class="error">✗ Apply failed!</p>
                    {% endif %}
                </div>
                <a href="/" class="btn">Back to Form</a>
            </div>
        </body>
        </html>
        ''', output=apply_status['output'], success=apply_status['success'], template_name=apply_status['template'])

@app.route('/download/<path:template_name>')
def download_template(template_name):
    template_path = os.path.join(TEMPLATES_DIR, template_name)
    
    if not os.path.exists(template_path):
        return "Template not found", 404
    
    # Create a ZIP file in memory
    zip_buffer = io.BytesIO()
    
    with zipfile.ZipFile(zip_buffer, 'w', zipfile.ZIP_DEFLATED) as zip_file:
        # Add all terraform files
        for root, dirs, files in os.walk(template_path):
            # Skip .terraform directory
            if '.terraform' in dirs:
                dirs.remove('.terraform')
            
            for file in files:
                # Skip terraform state files and lock files
                if file.endswith(('.tfstate', '.tfstate.backup', '.lock.hcl')):
                    continue
                    
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, template_path)
                zip_file.write(file_path, arcname)
        
        # Add a README file with the suggested message
        readme_content = f"""# {template_name}

This Terraform configuration was generated by the Databricks Workspace Creator tool.

## What was deployed:
- Databricks workspace with secure networking
- Virtual network with private and public subnets
- Network security groups and access controls
- Unity Catalog metastore (if configured)

## Next Steps:
We strongly recommend uploading this Terraform code to your Git repository for version control and continued management of your Databricks workspace.

### To use this code:
1. Initialize Terraform: `terraform init`
2. Review the plan: `terraform plan`
3. Apply changes: `terraform apply`
4. Destroy when needed: `terraform destroy`

### Important Notes:
- This code contains your terraform.tfvars with actual values
- Review and secure any sensitive information before committing to git
- Consider using Terraform workspaces for different environments
- Set up CI/CD pipelines for automated deployments

## Security Considerations:
- Never commit terraform.tfstate files to git
- Use remote state storage (Azure Storage, S3, etc.) for team collaboration
- Consider using Terraform Cloud or Enterprise for advanced features

Generated on: {time.strftime('%Y-%m-%d %H:%M:%S')}
"""
        zip_file.writestr('README.md', readme_content)
    
    zip_buffer.seek(0)
    
    # Return the ZIP file for download
    return send_file(
        zip_buffer,
        mimetype='application/zip',
        as_attachment=True,
        download_name=f'{template_name.replace(" ", "_")}_terraform_code.zip'
    )

if __name__ == '__main__':
    app.run(debug=True)