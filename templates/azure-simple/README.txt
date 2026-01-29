====================================================================
  AZURE DATABRICKS WORKSPACE DEPLOYMENT
====================================================================

Thank you for downloading this Databricks workspace template!

All deployment files are in the "terraform" folder.

QUICK START (3 steps):
----------------------

1. Open Terminal and navigate to the terraform folder:
   cd terraform

2. Create a Python virtual environment and install dependencies:
   python3 -m venv venv
   source venv/bin/activate     # On macOS/Linux
   venv\Scripts\activate        # On Windows
   pip install -r requirements.txt

3. Run the deployment UI:
   python deploy.py

   The web interface will open at http://localhost:8080
   where you can configure your Azure Databricks workspace
   and deploy it with a few clicks!

WHAT'S INCLUDED:
----------------
- terraform/        All Terraform configuration files
- terraform/deploy.py     Easy-to-use deployment web interface
- terraform/README.md     Detailed documentation

NEED HELP?
----------
Check terraform/README.md for detailed prerequisites,
configuration options, and troubleshooting guides.

====================================================================
