import os
import json
from typing import Dict, Any

class TemplateService:
    """Service for handling template operations"""

    @staticmethod
    def load_tfvars(tfvars_path: str) -> Dict[str, Any]:
        """Load and parse terraform.tfvars file"""
        data = {}
        with open(tfvars_path, 'r') as f:
            content = f.read()

        # Simple parsing for tfvars
        # Handle strings, booleans, maps
        lines = content.split('\n')
        i = 0
        while i < len(lines):
            line = lines[i].strip()
            if '=' in line and not line.startswith('#'):
                key, value = line.split('=', 1)
                key = key.strip()
                value = value.strip()
                if value.startswith('"') and value.endswith('"'):
                    data[key] = value[1:-1]
                elif value == 'true':
                    data[key] = True
                elif value == 'false':
                    data[key] = False
                elif value.startswith('{'):
                    # Parse map
                    map_lines = [value]
                    i += 1
                    while i < len(lines) and not lines[i].strip().endswith('}'):
                        map_lines.append(lines[i])
                        i += 1
                    if i < len(lines):
                        map_lines.append(lines[i])
                    map_content = '\n'.join(map_lines)
                    # Simple map parsing
                    map_data = {}
                    for mline in map_lines:
                        mline = mline.strip()
                        if ':' in mline and not mline.startswith('#'):
                            mkey, mval = mline.split(':', 1)
                            mkey = mkey.strip().strip('"')
                            mval = mval.strip().strip(',').strip('"')
                            map_data[mkey] = mval
                    data[key] = map_data
                else:
                    data[key] = value.strip('"')
            i += 1
        return data

    @staticmethod
    def save_tfvars(data: Dict[str, Any], tfvars_path: str):
        """Save data to terraform.tfvars file"""
        content = []
        for key, value in data.items():
            if isinstance(value, bool):
                content.append(f'{key} = {str(value).lower()}')
            elif isinstance(value, dict):
                content.append(f'{key} = {{')
                for k, v in value.items():
                    content.append(f'  "{k}": "{v}"')
                content.append('}')
            else:
                content.append(f'{key} = "{value}"')

        with open(tfvars_path, 'w') as f:
            f.write('\n'.join(content))

    @staticmethod
    def create_default_tfvars(tfvars_path: str) -> str:
        """Create a default terraform.tfvars content with empty/default values"""
        # Load the current tfvars to get the structure
        current_data = TemplateService.load_tfvars(tfvars_path)

        # Create default values based on the structure
        default_data = {}
        for key, value in current_data.items():
            if isinstance(value, bool):
                default_data[key] = False
            elif isinstance(value, dict):
                # For maps, keep the structure but empty values
                default_map = {}
                for k in value.keys():
                    if 'id' in k.lower() or 'key' in k.lower() or 'secret' in k.lower():
                        default_map[k] = ""  # Empty string for sensitive values
                    else:
                        default_map[k] = ""  # Empty string for other values
                default_data[key] = default_map
            else:
                # For strings, use empty or placeholder values
                if 'id' in key.lower() or 'key' in key.lower() or 'secret' in key.lower():
                    default_data[key] = ""  # Empty for sensitive values
                elif 'name' in key.lower():
                    default_data[key] = f"your-{key.replace('_', '-')}"
                elif 'location' in key.lower() or 'region' in key.lower():
                    default_data[key] = "East US" if 'azure' in tfvars_path.lower() else "us-east-1"
                elif 'cidr' in key.lower():
                    default_data[key] = "10.0.0.0/16"
                else:
                    default_data[key] = ""

        # Convert to tfvars format
        content = []
        for key, value in default_data.items():
            if isinstance(value, bool):
                content.append(f'{key} = {str(value).lower()}')
            elif isinstance(value, dict):
                content.append(f'{key} = {{')
                for k, v in value.items():
                    content.append(f'  "{k}": "{v}"')
                content.append('}')
            else:
                content.append(f'{key} = "{value}"')

        return '\n'.join(content)