# Databricks Workspace Creator

A Flask web application for creating Databricks workspaces using Terraform templates across multiple cloud providers.

## Architecture

This application follows a modular Flask architecture with the following structure:

```
app/
├── __init__.py          # Application factory
├── config.py            # Configuration management
├── models/              # Data models
│   ├── __init__.py
│   └── template.py      # Template and TemplateManager classes
├── services/            # Business logic services
│   ├── __init__.py
│   ├── template_service.py  # Terraform variable handling
│   └── terraform_service.py # Terraform execution
├── routes/              # Flask blueprints for routing
│   ├── __init__.py
│   ├── main.py          # Main routes (index)
│   ├── azure.py         # Azure-specific routes
│   ├── aws.py           # AWS routes (coming soon)
│   └── gcp.py           # GCP routes (coming soon)
├── static/              # Static assets
│   └── css/
│       └── styles.css   # Application styles
└── templates/           # Jinja2 templates
    ├── base.html        # Base template
    ├── index.html       # Home page
    ├── azure_templates.html
    ├── prerequisites.html
    ├── configure.html
    ├── status.html
    └── coming_soon.html
```

## Features

- **Multi-cloud support**: Azure (available), AWS/GCP (coming soon)
- **Template management**: Organized Terraform templates with metadata
- **Prerequisites guide**: Step-by-step setup instructions for new users
- **Async Terraform execution**: Background processing with status monitoring
- **Download functionality**: Export configured Terraform code
- **Modern UI**: Dark theme with responsive design

## Installation

1. Create a virtual environment:
```bash
python -m venv .venv
source .venv/bin/activate
```

2. Install dependencies:
```bash
pip install -r requirements.txt
```

3. Run the application:
```bash
python app.py
```

The application will be available at `http://localhost:5000`.

## Development

### Adding New Cloud Providers

1. Create a new blueprint in `app/routes/`
2. Add template files in `templates/terraform/`
3. Register the blueprint in `app/__init__.py`
4. Create corresponding HTML templates

### Code Organization Principles

- **Separation of Concerns**: UI, business logic, and data access are separated
- **Blueprint Organization**: Routes are organized by feature/domain
- **Service Layer**: Business logic is encapsulated in service classes
- **Template Inheritance**: Jinja2 templates use inheritance for consistency
- **Configuration Management**: All configuration is centralized

## API Endpoints

### Main Routes
- `GET /` - Home page with cloud selection

### Azure Routes
- `GET /azure` - Azure template selection
- `GET /prerequisites/<template_name>` - Prerequisites check
- `GET/POST /configure/<template_name>` - Configuration form
- `GET /status` - Terraform execution status
- `GET /download/<template_name>` - Download configured Terraform code

### AWS/GCP Routes (Coming Soon)
- `GET /aws` - AWS templates (placeholder)
- `GET /gcp` - GCP templates (placeholder)

## Configuration

The application uses environment variables for configuration:

- `FLASK_DEBUG`: Enable/disable debug mode (default: True)
- `SECRET_KEY`: Flask secret key (default: development key)

## Terraform Integration

The application integrates with Terraform through:

1. **Template Parsing**: Custom HCL parser for terraform.tfvars files
2. **Async Execution**: Background subprocess execution with status tracking
3. **Download Generation**: ZIP file creation with README and configuration

## Security Considerations

- Never commit terraform.tfstate files to version control
- Use remote state storage for team collaboration
- Review and secure sensitive information before downloading
- Consider using Terraform Cloud/Enterprise for advanced features

## Contributing

1. Follow the established code organization principles
2. Add tests for new functionality
3. Update documentation for API changes
4. Use meaningful commit messages

## License

This project is licensed under the MIT License.