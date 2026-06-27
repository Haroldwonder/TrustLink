terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# ── Networking ────────────────────────────────────────────────────────────────

data "aws_vpc" "default" {
  default = true
}

data "aws_subnets" "default" {
  filter {
    name   = "vpc-id"
    values = [data.aws_vpc.default.id]
  }
}

# ── Security Groups ───────────────────────────────────────────────────────────

resource "aws_security_group" "alb" {
  name        = "${var.name_prefix}-alb"
  description = "Allow HTTP/HTTPS inbound to ALB"
  vpc_id      = data.aws_vpc.default.id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "ecs" {
  name        = "${var.name_prefix}-ecs"
  description = "Allow traffic from ALB to ECS tasks"
  vpc_id      = data.aws_vpc.default.id

  ingress {
    from_port       = var.indexer_port
    to_port         = var.indexer_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "rds" {
  name        = "${var.name_prefix}-rds"
  description = "Allow PostgreSQL from ECS tasks"
  vpc_id      = data.aws_vpc.default.id

  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs.id]
  }
}

# ── RDS / PostgreSQL ──────────────────────────────────────────────────────────

resource "aws_db_subnet_group" "this" {
  name       = "${var.name_prefix}-rds"
  subnet_ids = data.aws_subnets.default.ids
}

resource "aws_db_instance" "indexer" {
  identifier             = "${var.name_prefix}-indexer"
  engine                 = "postgres"
  engine_version         = "15"
  instance_class         = var.db_instance_class
  allocated_storage      = var.db_allocated_storage
  db_name                = "trustlink"
  username               = var.db_username
  password               = var.db_password
  db_subnet_group_name   = aws_db_subnet_group.this.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  skip_final_snapshot    = var.environment != "mainnet"
  deletion_protection    = var.environment == "mainnet"

  tags = local.common_tags
}

# ── ECS Cluster & Fargate Task ────────────────────────────────────────────────

resource "aws_ecs_cluster" "this" {
  name = "${var.name_prefix}-cluster"
  tags = local.common_tags
}

resource "aws_cloudwatch_log_group" "indexer" {
  name              = "/ecs/${var.name_prefix}-indexer"
  retention_in_days = 30
}

resource "aws_iam_role" "ecs_task_execution" {
  name = "${var.name_prefix}-ecs-task-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution" {
  role       = aws_iam_role.ecs_task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

resource "aws_ecs_task_definition" "indexer" {
  family                   = "${var.name_prefix}-indexer"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.indexer_cpu
  memory                   = var.indexer_memory
  execution_role_arn       = aws_iam_role.ecs_task_execution.arn

  container_definitions = jsonencode([{
    name  = "indexer"
    image = var.indexer_image

    portMappings = [{
      containerPort = var.indexer_port
      protocol      = "tcp"
    }]

    environment = [
      { name = "DATABASE_URL", value = "postgresql://${var.db_username}:${var.db_password}@${aws_db_instance.indexer.address}:5432/trustlink" },
      { name = "STELLAR_NETWORK", value = var.stellar_network },
      { name = "CONTRACT_ID", value = var.contract_id },
      { name = "PORT", value = tostring(var.indexer_port) }
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = aws_cloudwatch_log_group.indexer.name
        "awslogs-region"        = var.aws_region
        "awslogs-stream-prefix" = "indexer"
      }
    }
  }])

  tags = local.common_tags
}

resource "aws_ecs_service" "indexer" {
  name            = "${var.name_prefix}-indexer"
  cluster         = aws_ecs_cluster.this.id
  task_definition = aws_ecs_task_definition.indexer.arn
  desired_count   = var.indexer_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = data.aws_subnets.default.ids
    security_groups  = [aws_security_group.ecs.id]
    assign_public_ip = true
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.indexer.arn
    container_name   = "indexer"
    container_port   = var.indexer_port
  }

  depends_on = [aws_lb_listener.http]

  tags = local.common_tags
}

# ── ALB ───────────────────────────────────────────────────────────────────────

resource "aws_lb" "this" {
  name               = "${var.name_prefix}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = data.aws_subnets.default.ids

  tags = local.common_tags
}

resource "aws_lb_target_group" "indexer" {
  name        = "${var.name_prefix}-indexer"
  port        = var.indexer_port
  protocol    = "HTTP"
  vpc_id      = data.aws_vpc.default.id
  target_type = "ip"

  health_check {
    path                = "/health"
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }
}

resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.this.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.indexer.arn
  }
}

# ── Locals ────────────────────────────────────────────────────────────────────

locals {
  common_tags = {
    Project     = "trustlink"
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

# ── Cost Monitoring ──────────────────────────────────────────────────────────

data "aws_caller_identity" "current" {}

resource "aws_budgets_budget" "trustlink" {
  name              = "${var.name_prefix}-${var.environment}-monthly"
  budget_type       = "MONTHLY"
  limit_unit        = "USD"
  limit_amount      = var.budget_threshold
  time_period_start = "2024-01-01_00:00"
  time_period_end   = "2099-12-31_23:59"

  tags = local.common_tags
}

resource "aws_budgets_budget_action" "alert" {
  budget_name        = aws_budgets_budget.trustlink.name
  action_id          = "${var.name_prefix}-alert"
  action_type        = "APPLY_SNS_NOTIFICATION"
  approval_model     = "AUTOMATIC"
  notification_type  = "FORECASTED"
  threshold          = 90
  threshold_type     = "PERCENTAGE"

  execute_capability = "AFTER_FORECASTED_AMOUNT"

  definition {
    sns_action_definition {
      topic_arn = aws_sns_topic.budget_alerts.arn
    }
  }

  depends_on = [aws_sns_topic_policy.budget_alerts]
}

resource "aws_sns_topic" "budget_alerts" {
  name = "${var.name_prefix}-budget-alerts"

  tags = local.common_tags
}

resource "aws_sns_topic_subscription" "budget_alerts_email" {
  topic_arn = aws_sns_topic.budget_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

resource "aws_sns_topic_policy" "budget_alerts" {
  arn = aws_sns_topic.budget_alerts.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "budgets.amazonaws.com"
      }
      Action   = "SNS:Publish"
      Resource = aws_sns_topic.budget_alerts.arn
      Condition = {
        StringEquals = {
          "aws:SourceAccount" = data.aws_caller_identity.current.account_id
        }
      }
    }]
  })
}
