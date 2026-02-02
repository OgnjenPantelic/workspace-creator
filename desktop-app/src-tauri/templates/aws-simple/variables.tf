variable "databricks_account_id" {
  description = "Databricks Account ID"
  type        = string
  sensitive   = true
}

variable "admin_user" {
  description = "Your email address (must already exist in your Databricks account)"
  type        = string
  validation {
    condition     = can(regex("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$", var.admin_user))
    error_message = "admin_user must be a valid email address"
  }
}

variable "prefix" {
  description = "Prefix for all resource names"
  type        = string
  default     = "databricks"
}

variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "cidr_block" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.4.0.0/16"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}

# Advanced: Use existing VPC (leave empty to create new)
variable "existing_vpc_id" {
  description = "Existing VPC ID (leave empty to create new VPC)"
  type        = string
  default     = ""
}

variable "existing_subnet_ids" {
  description = "Existing private subnet IDs (required if using existing VPC)"
  type        = list(string)
  default     = []
}

variable "existing_security_group_id" {
  description = "Existing security group ID (required if using existing VPC)"
  type        = string
  default     = ""
}

# Unity Catalog (leave empty to create new metastore)
variable "metastore_id" {
  description = "Existing Unity Catalog metastore ID (leave empty to create new)"
  type        = string
  default     = ""
}
