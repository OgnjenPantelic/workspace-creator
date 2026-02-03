# AWS Databricks Workspace - BYOVPC

Terraform template for deploying a Databricks workspace on AWS with customer-managed VPC.

## What Gets Deployed

- Databricks Workspace with BYOVPC
- VPC with private/public subnets across AZs
- Security Groups
- NAT Gateways
- IAM Roles and Policies
- S3 Root Storage Bucket (encrypted)
- Unity Catalog resources (optional)

## Prerequisites

- Terraform CLI
- AWS CLI (`aws configure` or SSO)
- Databricks service principal with account admin privileges

## Usage

This template is designed to be used via the Databricks Deployer desktop app.

For manual deployment:

```bash
terraform init
terraform plan -var-file="terraform.tfvars"
terraform apply -var-file="terraform.tfvars"
```

## Variables

| Variable | Description |
|----------|-------------|
| `databricks_account_id` | Databricks account ID |
| `databricks_client_id` | Service principal client ID |
| `databricks_client_secret` | Service principal secret |
| `prefix` | Resource name prefix |
| `region` | AWS region |
| `vpc_cidr` | VPC CIDR block |

## Security Notes

- Don't commit `terraform.tfstate` or `terraform.tfvars` with secrets
- Use S3 + DynamoDB for remote state
- Ensure IAM permissions to create VPCs, IAM roles, S3 buckets
