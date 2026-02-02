# Computed values from region and CIDR
data "aws_availability_zones" "available" {
  state = "available"
}

locals {
  # Use first 2 AZs in the region
  availability_zones = slice(data.aws_availability_zones.available.names, 0, 2)
  
  # Derive subnets from CIDR (e.g., 10.4.0.0/16 -> 10.4.1.0/24, 10.4.2.0/24, etc.)
  cidr_prefix = join(".", slice(split(".", var.cidr_block), 0, 2))
  
  private_subnets = [
    "${local.cidr_prefix}.1.0/24",
    "${local.cidr_prefix}.2.0/24"
  ]
  
  public_subnets = [
    "${local.cidr_prefix}.101.0/24",
    "${local.cidr_prefix}.102.0/24"
  ]
  
  # Standard Databricks egress ports
  egress_ports = [443, 3306, 6666, 8443, 8444, 8445, 8446, 8447, 8448, 8449, 8450, 8451]
}
