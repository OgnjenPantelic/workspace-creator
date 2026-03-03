
variable "project" {
  type    = string
  default = "<my-project-id>"
}

variable "use_existing_pas" {
  description = "Use existing private access settings instead of creating new ones"
  type        = bool
  default     = false
}

variable "existing_pas_id" {
  description = "ID of the existing Private Access Settings (required if use_existing_pas is true)"
  type        = string
  default     = ""
}
variable "databricks_google_service_account" {}

variable "google_region" {}
variable "google_project" {}

variable "databricks_account_id" {}
variable "key_name" {}
variable "keyring_name" {}
variable "use_existing_cmek" {
  description = "Use an existing CMEK resource instead of creating new key and keyring"
  type        = bool
  default     = false
}
variable "cmek_resource_id" {}

variable "workspace_pe" {}
variable "relay_pe" {}

# primary subnet providing ip addresses to PSC endpoints
variable "google_pe_subnet" {}

# Private ip address assigned to PSC endpoints
variable "relay_pe_ip_name" {}
variable "workspace_pe_ip_name" {}

variable "relay_service_attachment" {}
variable "workspace_service_attachment" {}

variable "account_console_url" {}
# IP addresses allowed to access the workspace
variable "ip_addresses" {
  type = list(string)
}

variable "workspace_name" {
  type        = string
  default     = "my-databricks-workspace"
  description = "The name of the Databricks workspace to create"
}