# Terraform UI

A simple web UI to configure Terraform variables and run `terraform apply`.

## Setup

1. Create a virtual environment:
   ```
   python3 -m venv venv
   ```

2. Activate the virtual environment:
   ```
   source venv/bin/activate
   ```

3. Install dependencies:
   ```
   pip install -r requirements.txt
   ```

## Running the UI

1. Ensure you are in the project directory with `terraform.tfvars`.

2. Run the Flask app:
   ```
   python app.py
   ```

3. Open your browser and go to `http://localhost:5000`.

4. Fill in the form with your desired values.

5. Click "Save and Apply" to update `terraform.tfvars` and run `terraform apply`.

## Features

- Parses existing `terraform.tfvars` and displays current values.
- Supports string, boolean, and map (JSON) variables.
- Saves changes back to `terraform.tfvars`.
- Runs `terraform apply -auto-approve` after saving.

## Notes

- The UI assumes a simple structure for `terraform.tfvars`. Complex nested structures may not be handled perfectly.
- Make sure Terraform is installed and configured properly.
- The app runs in debug mode; for production, set `debug=False`.