import os

class Config:
    """Application configuration"""
    SECRET_KEY = os.environ.get('SECRET_KEY') or 'dev-secret-key-change-in-production'
    DEBUG = os.environ.get('FLASK_DEBUG', 'True').lower() == 'true'

    # Template directories
    TEMPLATES_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'templates')
    TEMPLATE_TEMPLATES_DIR = os.path.join(TEMPLATES_DIR, 'terraform')

    # Flask template folder
    TEMPLATE_FOLDER = os.path.join(os.path.dirname(__file__), 'templates')

    # Thread-safe storage for apply status
    APPLY_STATUS = {}

    @staticmethod
    def init_app(app):
        pass