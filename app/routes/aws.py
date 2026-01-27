from flask import Blueprint, render_template, request, redirect, url_for, send_file
from urllib.parse import quote
import os
import zipfile
import io
import time
from ..models import TemplateManager
from ..services import TemplateService
from ..config import Config

aws_bp = Blueprint('aws', __name__)

template_manager = TemplateManager(Config.TEMPLATES_DIR)

@aws_bp.route('/aws', methods=['GET'])
def aws_templates():
    templates = template_manager.get_available_templates()
    # Filter only AWS templates
    aws_templates = [t for t in templates if 'aws' in t.name.lower()]
    return render_template('aws_templates.html', templates=aws_templates)

@aws_bp.route('/os-selection/<path:template_name>', methods=['GET'])
def os_selection(template_name):
    return render_template('os_selection.html', template_name=template_name, cloud='aws')

@aws_bp.route('/prerequisites/<path:template_name>/<os>', methods=['GET'])
def prerequisites_os(template_name, os):
    os_names = {
        'windows': 'Windows',
        'mac': 'macOS',
        'linux': 'Linux'
    }
    os_name = os_names.get(os, 'Unknown')
    return render_template('prerequisites_os.html', template_name=template_name, os=os, os_name=os_name, cloud='aws')

@aws_bp.route('/prerequisites/<path:template_name>', methods=['GET'])
def prerequisites(template_name):
    return render_template('prerequisites.html', template_name=template_name, cloud='aws')

@aws_bp.route('/configure/<path:template_name>', methods=['GET', 'POST'])
def configure(template_name):
    # This route is no longer used in the hosted version
    # Users download templates and configure locally
    return redirect(url_for('aws.prerequisites', template_name=template_name))

@aws_bp.route('/status')
def status():
    # This route is no longer used in the hosted version
    # Deployment happens locally
    return redirect(url_for('aws.aws_templates'))

@aws_bp.route('/download-template/<path:template_name>')
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
                    default_content = TemplateService.create_default_tfvars(file_path)
                    zip_file.writestr(arcname, default_content)
                else:
                    zip_file.write(file_path, arcname)

        # Include the local deployment UI script (deploy.py from workspace root)
        deploy_script_path = os.path.join(os.path.dirname(Config.TEMPLATES_DIR), 'deploy.py')
        if os.path.exists(deploy_script_path):
            zip_file.write(deploy_script_path, 'deploy.py')

    zip_buffer.seek(0)

    # Return the ZIP file for download
    safe_name = template_name.replace(' ', '_').replace('/', '_')
    timestamp = int(time.time())
    filename = f"{safe_name}_template_{timestamp}.zip"

    return send_file(
        zip_buffer,
        mimetype='application/zip',
        as_attachment=True,
        download_name=filename
    )


@aws_bp.route('/download/<path:template_name>')
def download_template(template_name):
    """Alias for download-template for backward compatibility"""
    return download_clean_template(template_name)