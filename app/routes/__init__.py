from .main import main_bp
from .azure import azure_bp
from .aws import aws_bp
from .gcp import gcp_bp

__all__ = ['main_bp', 'azure_bp', 'aws_bp', 'gcp_bp']