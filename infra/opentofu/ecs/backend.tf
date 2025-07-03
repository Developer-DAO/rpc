terraform {
      backend "s3" {
          bucket         = "dd-test-rpc-terraform-state"
          key            = "ecs/terraform.tfstate"
          region         = "us-east-2"
          encrypt        = true
      }

  required_version = ">= 1.0.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.region
}

