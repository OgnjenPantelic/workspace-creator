"""
Databricks App - Databricks Deployer Download Page
Simple Flask server to serve the static download page
"""

from flask import Flask, send_from_directory, jsonify
import os

app = Flask(__name__, static_folder='.')

@app.route('/')
def index():
    """Serve the main index page"""
    return send_from_directory('.', 'index.html')

@app.route('/<path:path>')
def serve_static(path):
    """Serve static files (CSS, JS, etc.)"""
    return send_from_directory('.', path)

@app.route('/health')
def health():
    """Health check endpoint for Databricks"""
    return jsonify({"status": "healthy"}), 200

if __name__ == '__main__':
    # Get port from environment variable or default to 8080
    port = int(os.environ.get('PORT', 8080))
    
    # Run the Flask app
    app.run(host='0.0.0.0', port=port, debug=False)
