import os
from typing import Dict, Any, List, Optional

class Template:
    """Represents a Terraform template"""

    def __init__(self, name: str, path: str, description: str = None):
        self.name = name
        self.path = path
        self.description = description or self._get_default_description()

    def _get_default_description(self) -> str:
        """Get default description for template"""
        descriptions = {
            'azure-simple': 'Deploy a secure Databricks workspace with VNet injection for enhanced network isolation and security.',
            'aws-simple': 'Deploy a Databricks workspace on AWS with BYOVPC for secure networking and custom VPC configuration.'
        }
        return descriptions.get(self.name, f'Deploy {self.name} workspace')

    @property
    def directory(self) -> str:
        """Get template directory path (alias for path)"""
        return self.path

    @property
    def tfvars_path(self) -> str:
        """Get path to terraform.tfvars file"""
        return os.path.join(self.path, 'terraform', 'terraform.tfvars')

    def exists(self) -> bool:
        """Check if template exists"""
        return os.path.exists(self.path) and os.path.exists(self.tfvars_path)

class TemplateManager:
    """Manages Terraform templates"""

    def __init__(self, templates_dir: str):
        self.templates_dir = templates_dir

    def get_available_templates(self) -> List[Template]:
        """Get list of available templates"""
        templates = []
        if os.path.exists(self.templates_dir):
            for item in os.listdir(self.templates_dir):
                template_path = os.path.join(self.templates_dir, item)
                terraform_subdir = os.path.join(template_path, 'terraform')
                # Check if terraform subfolder exists with terraform.tfvars
                if os.path.isdir(template_path) and os.path.exists(os.path.join(terraform_subdir, 'terraform.tfvars')):
                    template = Template(item, template_path)
                    templates.append(template)
        return templates

    def get_template(self, name: str) -> Optional[Template]:
        """Get a specific template by name"""
        templates = self.get_available_templates()
        for template in templates:
            if template.name == name:
                return template
        return None