from flask import Blueprint, render_template

gcp_bp = Blueprint('gcp', __name__)

@gcp_bp.route('/gcp', methods=['GET'])
def gcp_templates():
    return render_template('coming_soon.html', cloud_name='Google Cloud Platform')