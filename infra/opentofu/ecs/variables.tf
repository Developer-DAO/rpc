variable "region" {
  description = "The AWS region to deploy the VPC in."
  default     = "us-east-2"
  type        = string
}

variable "rpc_image" {
  description = "The image tag or URI for the dd-rpc container."
  type        = string
  default     = "ghcr.io/developer-dao/rpc:latest"
}