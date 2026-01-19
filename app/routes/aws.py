from flask import Blueprint, render_template

aws_bp = Blueprint('aws', __name__)

@aws_bp.route('/aws', methods=['GET'])
def aws_templates():
    return render_template('coming_soon.html', cloud_name='AWS')