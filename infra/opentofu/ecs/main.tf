# Reference VPC outputs from the vpc folder's state
# TODO: Migrate to S3 when AWS account and S3 bucket is set up
data "terraform_remote_state" "vpc" {
  backend = "s3"
  config = {
      bucket = "dd-cloud-terraform-state"
      key    = "vpc/terraform.tfstate"
      region = "us-east-2"
  }
}

data "aws_ami" "ecs_ami" {
  most_recent = true
  owners      = ["amazon"]
  filter {
    name   = "name"
    values = ["amzn2-ami-ecs-hvm-*-x86_64-ebs"]
  }
}

module "ecs" {
  source = "terraform-aws-modules/ecs/aws"
  version = "5.12.1"

  cluster_name = "rpc-ecs-cluster"

  cluster_configuration = {
    execute_command_configuration = {
      logging = "OVERRIDE"
      log_configuration = {
        cloud_watch_log_group_name = "/aws/ecs/aws-ec2"
      }
    }
  }

  # Capacity provider - autoscaling groups
  default_capacity_provider_use_fargate = false
  autoscaling_capacity_providers = {
    # On-demand instances
    rpc_ec2 = {
      auto_scaling_group_arn         = module.autoscaling["rpc_ec2"].autoscaling_group_arn
      managed_termination_protection = "ENABLED"

      managed_scaling = {
        desired_size = 1
        maximum_scaling_step_size = 1
        minimum_scaling_step_size = 1
        status                    = "ENABLED"
        target_capacity           = 60
      }

      default_capacity_provider_strategy = {
        weight = 60
        base   = 20
      }
    }
  }

  services = {
    dd-rpc = {
      cpu    = 4096
      memory = 8192

      # Container definition(s)
      container_definitions = {
        path = {
          cpu                = 2048
          memory             = 4096
          essential          = true
          image              = "ghcr.io/pokt-network/path:main"
          memory_reservation = 50
          port_mappings = [
            {
              name          = "path"
              containerPort = 3069
              protocol      = "tcp"
            }
          ]
          secrets = [
            {
              name      = "GATEWAY_CONFIG"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:GatewayConfig-hQSJJs"
            }
          ]
        }
        
        rpc = {
          cpu       = 2048
          memory    = 4096
          essential = true
          image     = var.rpc_image

          repository_credentials = {
            credentialsParameter = "arn:aws:secretsmanager:us-east-2:975950814568:secret:GhcrCredentials-j8eElR"
          }

          port_mappings = [
            {
              name          = "dd-rpc"
              containerPort = 3000
              protocol      = "tcp"
            }
          ]

          memory_reservation = 100
          secrets = [
            {
              name      = "SMTP_USERNAME"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:SMTP_USERNAME::" 
            },
            {
              name      = "SMTP_PASSWORD"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:SMTP_PASSWORD::"
            },
            {
              name      = "JWT_KEY"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:JWT_KEY::" 
            },
            {
              name      = "DATABASE_URL"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:DATABASE_URL::" 
            },
            {
              name      = "SEPOLIA_ENDPOINT"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:SEPOLIA_ENDPOINT::"
            },
            {
              name      = "ETHEREUM_ENDPOINT"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:ETHEREUM_ENDPOINT::"
            },
            {
              name      = "SEPOLIA_WS"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:SEPOLIA_WS::"
            },
            {
              name      = "D_D_CLOUD_API_KEY"
              valueFrom = "arn:aws:secretsmanager:us-east-2:975950814568:secret:dd-cloud-nyylCQ:D_D_CLOUD_API_KEY::"
            }
          ]
        }
      }

      load_balancer = {
        service = {
          target_group_arn = module.alb.target_groups["rpc"].arn
          container_name   = "rpc"
          container_port   = 3000
        }
      }

      subnet_ids = data.terraform_remote_state.vpc.outputs.private_subnets
      security_group_rules = {
        alb_ingress_3000 = {
          type                     = "ingress"
          from_port                = 3000
          to_port                  = 3000
          protocol                 = "tcp"
          description              = "Service port"
          source_security_group_id = module.alb.security_group_id
        }
        egress_all = {
          type        = "egress"
          from_port   = 0
          to_port     = 0
          protocol    = "-1"
          cidr_blocks = ["0.0.0.0/0"]
        }
      }
    }
  }

  tags = local.tags
}

# Supporting resources
data "aws_ssm_parameter" "ecs_optimized_ami" {
  name = "/aws/service/ecs/optimized-ami/amazon-linux-2/recommended"
}

resource "aws_acm_certificate" "cert" {
  domain_name       = "api.cloud.developerdao.com"
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_acm_certificate_validation" "validation" {
  certificate_arn = aws_acm_certificate.cert.arn
}

module "alb" {
  source  = "terraform-aws-modules/alb/aws"
  version = "~> 9.0"
  name    = "${local.name}-alb"
  load_balancer_type = "application"
  vpc_id  = data.terraform_remote_state.vpc.outputs.vpc_id
  subnets = data.terraform_remote_state.vpc.outputs.public_subnets

  # Security Group
 security_group_ingress_rules = {
    all_http = {
      from_port   = 80
      to_port     = 80
      ip_protocol = "tcp"
      description = "HTTP web traffic"
      cidr_ipv4   = "0.0.0.0/0"
    }
    all_https = {
      from_port   = 443
      to_port     = 443
      ip_protocol = "tcp"
      description = "HTTPS web traffic"
      cidr_ipv4   = "0.0.0.0/0"
    }
  }
  security_group_egress_rules = {
    all = {
      ip_protocol = "-1"
      cidr_ipv4   = data.terraform_remote_state.vpc.outputs.vpc_cidr_block
    }
  }

  listeners = {
    rpc_http = {
      port     = 80
      protocol = "HTTP"

      redirect = {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }
    rpc_https = {
      port     = 443
      protocol = "HTTPS"
      certificate_arn = aws_acm_certificate.cert.arn

      forward = {
        target_group_key = "rpc"
      }
    }
  }

  target_groups = {
    rpc = {
      backend_protocol                  = "HTTP"
      backend_port                      = 3000
      target_type                       = "ip"
      deregistration_delay              = 5
      load_balancing_cross_zone_enabled = true

      health_check = {
        enabled             = true
        healthy_threshold   = 5
        interval            = 30
        matcher             = "200"
        path                = "/api/checkhealth"
        port                = "traffic-port"
        protocol            = "HTTP"
        timeout             = 5
        unhealthy_threshold = 2
      }

      # Theres nothing to attach here in this definition. Instead,
      # ECS will attach the IPs of the tasks to this target group
      create_attachment = false
    }
  }

  tags = local.tags

  depends_on = [
    aws_acm_certificate_validation.validation
  ]
}

module "autoscaling" {
  source  = "terraform-aws-modules/autoscaling/aws"
  version = "~> 6.5"

  for_each = {
    # On-demand instances
    rpc_ec2 = {
      instance_type              = "c6a.xlarge"
      use_mixed_instances_policy = true
      mixed_instances_policy     = {
        instances_distribution = {
          on_demand_base_capacity                  = 0
          on_demand_percentage_above_base_capacity = 25
          spot_allocation_strategy                 = "price-capacity-optimized"
        }
      }
      user_data                  = <<-EOT
        #!/bin/bash
        cat <<'EOF' >> /etc/ecs/ecs.config
        ECS_CLUSTER=${local.name}
        ECS_LOGLEVEL=debug
        ECS_CONTAINER_INSTANCE_TAGS=${jsonencode(local.tags)}
        ECS_ENABLE_TASK_IAM_ROLE=true
        EOF
      EOT
    }
  }

  name = "${local.name}-${each.key}"

  image_id      = jsondecode(data.aws_ssm_parameter.ecs_optimized_ami.value)["image_id"]
  instance_type = each.value.instance_type

  security_groups                 = [module.autoscaling_sg.security_group_id]
  user_data                       = base64encode(each.value.user_data)
  ignore_desired_capacity_changes = true

  create_iam_instance_profile = true
  iam_role_name               = local.name
  iam_role_description        = "ECS role for ${local.name}"
  iam_role_policies = {
    AmazonEC2ContainerServiceforEC2Role = "arn:aws:iam::aws:policy/service-role/AmazonEC2ContainerServiceforEC2Role"
    AmazonSSMManagedInstanceCore        = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
  }

  vpc_zone_identifier = data.terraform_remote_state.vpc.outputs.private_subnets
  capacity_rebalance  = true
  health_check_type   = "EC2"
  min_size            = 1
  max_size            = 2
  desired_capacity    = 1 

  # https://github.com/hashicorp/terraform-provider-aws/issues/12582
  autoscaling_group_tags = {
    AmazonECSManaged = true
    propagate_at_launch = true
  }

  # Required for managed_termination_protection = "ENABLED"
  protect_from_scale_in = true

  # Spot instances
  use_mixed_instances_policy = each.value.use_mixed_instances_policy
  mixed_instances_policy     = each.value.mixed_instances_policy

  tags = local.tags
}

module "autoscaling_sg" {
  source  = "terraform-aws-modules/security-group/aws"
  version = "~> 5.0"

  name        = local.name
  description = "Autoscaling group security group"
  vpc_id      = data.terraform_remote_state.vpc.outputs.vpc_id

  computed_ingress_with_source_security_group_id = [
    {
      rule                     = "http-80-tcp"
      source_security_group_id = module.alb.security_group_id
    }
  ]
  number_of_computed_ingress_with_source_security_group_id = 1

  egress_rules = ["all-all"]

  tags = local.tags
}
