from flask import Flask, render_template_string, request, redirect, url_for
import subprocess
import os
import re
import threading
import time

app = Flask(__name__)

TFVARS_FILE = 'terraform.tfvars'

# Global variable to store apply status
apply_status = {'running': False, 'output': '', 'success': None}

def load_tfvars():
    data = {}
    with open(TFVARS_FILE, 'r') as f:
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

def save_tfvars(data):
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
    with open(TFVARS_FILE, 'w') as f:
        f.write('\n'.join(content))

def run_terraform_apply_async():
    global apply_status
    apply_status['running'] = True
    apply_status['output'] = ''
    apply_status['success'] = None
    try:
        process = subprocess.Popen(['terraform', 'apply', '-auto-approve'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=os.getcwd())
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

@app.route('/', methods=['GET', 'POST'])
def index():
    global apply_status
    if request.method == 'POST':
        # Update tfvars
        data = load_tfvars()
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
        save_tfvars(data)
        
        # Run terraform apply in background
        threading.Thread(target=run_terraform_apply_async).start()
        
        return redirect(url_for('status'))
    
    data = load_tfvars()
    return render_template_string('''
    <!DOCTYPE html>
    <html>
    <head>
        <title>Terraform Configuration</title>
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
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Terraform Variables Configuration</h1>
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
    ''', data=data)

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
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Terraform Apply Result</h1>
                <div class="status">
                    <pre>{{ output }}</pre>
                    {% if success %}
                    <p class="success">✓ Apply successful!</p>
                    {% else %}
                    <p class="error">✗ Apply failed!</p>
                    {% endif %}
                </div>
                <a href="/" class="btn">Back to Form</a>
            </div>
        </body>
        </html>
        ''', output=apply_status['output'], success=apply_status['success'])

if __name__ == '__main__':
    app.run(debug=True)