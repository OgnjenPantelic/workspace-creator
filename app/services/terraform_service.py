import subprocess
import threading
import os
from typing import Dict, Any, Callable
from flask import current_app

class TerraformService:
    """Service for handling Terraform operations"""

    @staticmethod
    def run_apply_async(template_name: str, template_path: str, callback: Callable = None):
        """Run terraform apply asynchronously"""
        def _run_apply():
            apply_status = current_app.config['APPLY_STATUS']
            apply_status['running'] = True
            apply_status['output'] = ''
            apply_status['success'] = None
            apply_status['template'] = template_name

            try:
                process = subprocess.Popen(
                    ['terraform', 'apply', '-auto-approve'],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    cwd=template_path
                )

                while True:
                    output = process.stdout.readline()
                    if output == '' and process.poll() is not None:
                        break
                    if output:
                        apply_status['output'] += output

                apply_status['success'] = process.returncode == 0

            except Exception as e:
                apply_status['output'] = str(e)
                apply_status['success'] = False

            apply_status['running'] = False

            if callback:
                callback()

        thread = threading.Thread(target=_run_apply)
        thread.daemon = True
        thread.start()

    @staticmethod
    def get_apply_status() -> Dict[str, Any]:
        """Get current apply status"""
        return current_app.config.get('APPLY_STATUS', {
            'running': False,
            'output': '',
            'success': None,
            'template': None
        })