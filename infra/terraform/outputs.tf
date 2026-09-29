output "alb_dns_name" {
  description = "DNS name of the Application Load Balancer"
  value       = aws_lb.this.dns_name
}

output "graphql_endpoint" {
  description = "GraphQL HTTP and WebSocket endpoint"
  value       = "http://${aws_lb.this.dns_name}:${var.gql_port}/graphql"
}

output "rds_endpoint" {
  description = "RDS PostgreSQL endpoint"
  value       = aws_db_instance.indexer.address
}

output "ecs_cluster_name" {
  description = "ECS cluster name"
  value       = aws_ecs_cluster.this.name
}

output "budget_alert_topic_arn" {
  description = "SNS topic ARN for budget alerts"
  value       = aws_sns_topic.budget_alerts.arn
}

output "budget_threshold_usd" {
  description = "Monthly budget threshold in USD"
  value       = var.budget_threshold
}
