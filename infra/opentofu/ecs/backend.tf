terraform {
  #     backend "s3" { # TODO: Migrate to S3 when AWS account and S3 bucket is set up
  #         bucket         = "dd-rpc-terraform-state"
  #         key            = "ecs/terraform.tfstate"
  #         region         = var.region
  #         encrypt        = true
  #     }
  backend "local" {}

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

