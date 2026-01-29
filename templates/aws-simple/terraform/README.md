# AWS Databricks Workspace - Standard BYOVPC

This Terraform template deploys a production-grade Databricks workspace on AWS with customer-managed VPC.

## �� Quick Start (2 minutes)

### Step 1: Create Virtual Environment (First Time Only)
```bash
# macOS/Linux
python3 -m venv venv
source venv/bin/activate

# Windows (Command Prompt)
python -m venv venv
venv\Scripts\activate

# Windows (PowerShell)
python -m venv venv
venv\Scripts\Activate.ps1
```

### Step 2: Install Python Dependencies
```bash
# After activating virtual environment above
pip install -r requirements.txt
```

### Step 3: Run the Deployment UI
```bash
# Make sure virtual environment is activated (you should see (venv) in your prompt)
python deploy.py
```

The web interface will automatically open at **http://localhost:8081**

### Step 4: Configure and Deploy
1. Fill in your configuration values in the web UI
2. Click "Save Configuration"
3. Click "Run Terraform Plan" to preview changes
4. Click "Apply Configuration" to deploy

That's it! ✨

---

## 📋 Prerequisites

Before running the deployment, ensure you have:

### Required Software
- **Python 3.7+** - [Download here](https://www.python.org/downloads/)
- **Terraform CLI** - [Download here](https://www.terraform.io/downloads)
- **AWS CLI** - [Download here](https://aws.amazon.com/cli/)

### AWS Setup
1. Configure AWS CLI with your credentials:
   ```bash
   aws configure
   ```
   
2. Verify your AWS credentials are working:
   ```bash
   aws sts get-caller-identity
   ```

### Databricks Account
You'll need the following from your Databricks account:
- **Account ID** (found in account console)
- **Client ID** (from service principal)
- **Client Secret** (from service principal)

---

## 🏗️ What Will Be Deployed

This template creates:
- **Databricks Workspace** with customer-managed VPC (BYOVPC)
- **VPC** with private and public subnets across availability zones
- **Security Groups** controlling inbound/outbound traffic
- **NAT Gateways** for secure outbound internet access from private subnets
- **IAM Roles and Policies** for fine-grained access control
- **S3 Root Storage Bucket** with encryption and logging
- **Unity Catalog Configuration** (optional, if metastore configured)

---

## 🎯 Deployment Options

### Option 1: Web UI (Recommended)
Use the local deployment UI as described in Quick Start above.

### Option 2: Manual Terraform Commands
If you prefer command-line:

```bash
# Initialize Terraform
terraform init

# Review the deployment plan
terraform plan

# Apply the configuration
terraform apply
```

### Option 3: Edit Configuration File Manually
Edit `terraform.tfvars` directly, then run terraform commands.

---

## 🔒 Security Best Practices

- **Never commit** `terraform.tfstate` or `terraform.tfvars` with secrets to version control
- Use **remote state storage** (S3 with DynamoDB for locking) for team collaboration
- Review all configuration before running `terraform apply`
- Ensure you have appropriate **AWS IAM permissions** to create VPCs, IAM roles, S3 buckets, etc.
- Consider using **AWS Secrets Manager** or **Parameter Store** for sensitive values

---

## 🆘 Troubleshooting

### Port 8081 already in use
Kill the process using the port:
```bash
# macOS/Linux
lsof -ti:8081 | xargs kill -9

# Windows
netstat -ano | findstr :8081
taskkill /PID <PID> /F
```

### Flask not found or "externally-managed-environment" error
Make sure you created and activated a virtual environment (see Step 1 above):
```bash
python3 -m venv venv
source venv/bin/activate  # macOS/Linux
pip install -r requirements.txt
```

### AWS CLI not configured
Run `aws configure` and provide your access key, secret key, and default region.

### Terraform errors
- Ensure Terraform is installed: `terraform version`
- Run `terraform init` in the template directory
- Check AWS IAM permissions
- Verify AWS CLI credentials: `aws sts get-caller-identity`

---

## 📚 Additional Resources

- [Databricks on AWS Documentation](https://docs.databricks.com/administration-guide/cloud-configurations/aws/index.html)
- [Terraform AWS Provider Docs](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)
- [AWS CLI Reference](https://docs.aws.amazon.com/cli/)

---

**Questions or issues?** Contact your Databricks representative or check the Databricks documentation.
