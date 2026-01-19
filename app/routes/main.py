from flask import Blueprint, render_template
from ..models import TemplateManager
from ..config import Config

main_bp = Blueprint('main', __name__)

@main_bp.route('/', methods=['GET'])
def index():
    return render_template('index.html')