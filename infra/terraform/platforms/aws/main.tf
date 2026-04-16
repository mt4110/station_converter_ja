
terraform {
  required_version = ">= 1.6.0"

  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}

provider "aws" {
  region = var.region
}

module "service_contract" {
  source          = "../../modules/service_contract"
  name            = var.name
  environment     = var.environment
  image           = var.image
  api_port        = var.api_port
  database_engine = var.database_engine
  redis_enabled   = var.redis_enabled
}
