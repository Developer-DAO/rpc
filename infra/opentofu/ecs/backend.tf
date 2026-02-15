terraform {
      backend "s3" {
          bucket         = "dd-cloud-terraform-state"
          key            = "ecs/terraform.tfstate"
          region         = "us-east-2"
          encrypt        = true
      }

  required_version = ">= 1.5.7"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.32.1"
    }
  }
}

provider "aws" {
  region = var.region
}

