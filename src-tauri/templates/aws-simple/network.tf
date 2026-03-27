locals {
  create_vpc = var.create_new_vpc
}

module "vpc" {
  count   = local.create_vpc ? 1 : 0
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.1.1"

  name = "${var.prefix}-vpc"
  cidr = var.cidr_block
  azs  = local.availability_zones

  enable_dns_hostnames   = true
  enable_nat_gateway     = true
  single_nat_gateway     = true
  one_nat_gateway_per_az = false
  create_igw             = true

  private_subnet_names = [for az in local.availability_zones : "${var.prefix}-private-${az}"]
  private_subnets      = local.private_subnets

  public_subnet_names = ["${var.prefix}-public-nat"]
  public_subnets      = local.public_subnets

  tags = var.tags
}

# Create dedicated security group for Databricks (when creating VPC)
resource "aws_security_group" "databricks" {
  count       = local.create_vpc ? 1 : 0
  name        = "${var.prefix}-databricks-sg"
  description = "Security group for Databricks workspace"
  vpc_id      = module.vpc[0].vpc_id
  tags        = merge(var.tags, { Name = "${var.prefix}-databricks-sg" })
}

# VPC Endpoints: S3 (Gateway), STS and Kinesis Streams (Interface)
# S3 gateway endpoint routes S3 traffic over the AWS backbone instead of the internet.
# STS and Kinesis interface endpoints enable private connectivity for token service and streaming.
module "vpc_endpoints" {
  count   = local.create_vpc ? 1 : 0
  source  = "terraform-aws-modules/vpc/aws//modules/vpc-endpoints"
  version = "5.1.1"

  vpc_id = module.vpc[0].vpc_id

  endpoints = {
    s3 = {
      service         = "s3"
      service_type    = "Gateway"
      route_table_ids = module.vpc[0].private_route_table_ids
      tags = {
        Name    = "${var.prefix}-s3-vpc-endpoint"
        Project = var.prefix
      }
    }
    sts = {
      service             = "sts"
      private_dns_enabled = true
      subnet_ids          = module.vpc[0].private_subnets
      tags = {
        Name    = "${var.prefix}-sts-vpc-endpoint"
        Project = var.prefix
      }
    }
    kinesis-streams = {
      service             = "kinesis-streams"
      private_dns_enabled = true
      subnet_ids          = module.vpc[0].private_subnets
      tags = {
        Name    = "${var.prefix}-kinesis-vpc-endpoint"
        Project = var.prefix
      }
    }
  }

  tags = var.tags
}

# Resolved values (created or existing)
locals {
  vpc_id             = local.create_vpc ? module.vpc[0].vpc_id : var.existing_vpc_id
  subnet_ids         = local.create_vpc ? module.vpc[0].private_subnets : var.existing_subnet_ids
  security_group_id  = local.create_vpc ? aws_security_group.databricks[0].id : var.existing_security_group_id
}
