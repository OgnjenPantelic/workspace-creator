from flask import Flask
from .config import Config
from .routes import main_bp, azure_bp, aws_bp, gcp_bp

def create_app():
    app = Flask(__name__, template_folder=Config.TEMPLATE_FOLDER)
    app.config.from_object(Config)

    # Initialize config
    Config.init_app(app)

    # Register blueprints
    app.register_blueprint(main_bp)
    app.register_blueprint(azure_bp)
    app.register_blueprint(aws_bp)
    app.register_blueprint(gcp_bp)

    return app