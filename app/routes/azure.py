from flask import Blueprint, render_template, request, redirect, url_for, send_file
from urllib.parse import quote
import os
import zipfile
import io
import time
from ..models import TemplateManager
from ..services import TemplateService
from ..config import Config

azure_bp = Blueprint('azure', __name__)

template_manager = TemplateManager(Config.TEMPLATES_DIR)

@azure_bp.route('/azure', methods=['GET'])
def azure_templates():
    templates = template_manager.get_available_templates()
    # Filter only Azure templates
    azure_templates = [t for t in templates if 'azure' in t.name.lower()]
    return render_template('azure_templates.html', templates=azure_templates)

@azure_bp.route('/os-selection/<path:template_name>', methods=['GET'])
def os_selection(template_name):
    return render_template('os_selection.html', template_name=template_name)

@azure_bp.route('/prerequisites/<path:template_name>/<os>', methods=['GET'])
def prerequisites_os(template_name, os):
    os_names = {
        'windows': 'Windows',
        'mac': 'macOS',
        'linux': 'Linux'
    }
    os_name = os_names.get(os, 'Unknown')
    return render_template('prerequisites_os.html', template_name=template_name, os=os, os_name=os_name)

@azure_bp.route('/prerequisites/<path:template_name>', methods=['GET'])
def prerequisites(template_name):
    return render_template('prerequisites.html', template_name=template_name)

@azure_bp.route('/configure/<path:template_name>', methods=['GET', 'POST'])
def configure(template_name):
    # This route is no longer used in the hosted version
    # Users download templates and configure locally
    return redirect(url_for('azure.prerequisites', template_name=template_name))

@azure_bp.route('/status')
def status():
    # This route is no longer used in the hosted version
    # Deployment happens locally
    return redirect(url_for('azure.azure_templates'))

@azure_bp.route('/download-template/<path:template_name>')
def download_clean_template(template_name):
    """Download a clean template without pre-populated values"""
    template = template_manager.get_template(template_name)
    if not template:
        return "Template not found", 404

    # Create a ZIP file in memory
    zip_buffer = io.BytesIO()

    with zipfile.ZipFile(zip_buffer, 'w', zipfile.ZIP_DEFLATED) as zip_file:
        # Add all files from the template directory
        for root, dirs, files in os.walk(template.path):
            # Skip .terraform directory
            if '.terraform' in dirs:
                dirs.remove('.terraform')

            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, template.path)

                # If it's terraform.tfvars, create a clean version with default values
                if file == 'terraform.tfvars':
                    # Create default tfvars content
                    default_content = TemplateService.create_default_tfvars(template.tfvars_path)
                    zip_file.writestr(arcname, default_content)
                else:
                    zip_file.write(file_path, arcname)

        # Include the template-specific deployment UI script
        template_deploy_path = os.path.join(template.directory, 'deploy.py')
        if os.path.exists(template_deploy_path):
            with open(template_deploy_path, 'r') as f:
                zip_file.writestr('deploy.py', f.read())
        else:
            # Fallback: create a simple deploy script
            simple_deploy = '''#!/usr/bin/env python3
import os
print("Local deployment UI not available. Please run terraform commands manually:")
print("terraform init")
print("terraform plan")
print("terraform apply")
'''
            zip_file.writestr('deploy.py', simple_deploy)

        # Add a README file with deployment instructions
        readme_content = f"""# {template_name}

This Terraform configuration was downloaded from the Databricks Workspace Creator tool.

## What will be deployed:
- Databricks workspace with secure networking
- Virtual network with private and public subnets
- Network security groups and access controls
- Unity Catalog metastore (if configured)

## Quick Start with Local UI:

### Option 1: Use the Local Web Interface (Recommended)
1. Make sure you have Python 3 installed
2. Run: `python deploy.py`
3. Open your browser to: http://localhost:8080
4. Configure your variables in the web interface
5. Click the deployment buttons

### Option 2: Manual Terraform Commands
```bash
# Initialize Terraform
terraform init

# Review the deployment plan
terraform plan

# Apply the configuration
terraform apply
```

### Option 3: Use the Local UI for Configuration Only
```bash
# Start the local configuration UI
python deploy.py

# Then run terraform commands manually in another terminal
terraform init
terraform plan
terraform apply
```

## Setup Instructions:

### 1. Prerequisites
Make sure you have installed:
- Terraform CLI (https://www.terraform.io/downloads)
- Azure CLI (for Azure templates): `az login`
- AWS CLI (for AWS templates): `aws configure`
- Python 3 (for the local UI)

### 2. Configure Variables
Edit the `terraform.tfvars` file with your specific values:
- Cloud provider credentials
- Databricks account information
- Network settings
- Resource names and locations

### 3. Deploy
Use either the local web UI or run terraform commands directly.

## Important Notes:
- Review all variables in terraform.tfvars before deploying
- Ensure you have appropriate permissions in your cloud provider
- Consider using Terraform workspaces for different environments
- Never commit terraform.tfstate files to version control

## Security Considerations:
- Use remote state storage (Azure Storage, S3, etc.) for team collaboration
- Consider using Terraform Cloud or Enterprise for advanced features
- Review and secure any sensitive information

## Local UI Features:
- Web-based configuration editor
- One-click terraform init, plan, apply, and destroy
- Real-time command output
- Form validation and error handling

Downloaded on: {time.strftime('%Y-%m-%d %H:%M:%S')}
"""
        zip_file.writestr('README.md', readme_content)

    zip_buffer.seek(0)

    # Return the ZIP file for download
    return send_file(
        zip_buffer,
        mimetype='application/zip',
        as_attachment=True,
        download_name=f'{template_name.replace(" ", "_")}_template.zip'
    )

@azure_bp.route('/download/<path:template_name>')
def download_template(template_name):
    # This route is no longer used - use download_clean_template instead
    return redirect(url_for('azure.download_clean_template', template_name=template_name))