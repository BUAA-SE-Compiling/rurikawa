# Rurikawa

Rurikawa is a simple Docker-based build & judge system for complex multi-file projects.

Rurikawa 是一个简易的自动评测系统，主要面向多文件项目和较为复杂的构建、评测步骤。

# 运行

- 阅读 `dev.docker-compose.yml` 并进行相应修改

- `docker-compose -f dev.docker-compose.yml up`

## Coordinator

运行（或购买相关服务）：

- PostgreSQL 数据库
- 兼容 Amazon S3 的对象存储服务（如 Minio）

将以上服务配置填写在 `appsettings.json` 中：

```json
{
  "Logging": {
    "LogLevel": {
      "Default": "Information",
      "Microsoft": "Warning",
      "Microsoft.Hosting.Lifetime": "Information"
    }
  },

  // Your postgresql database
  "pgsqlLink": "Host=<host>;Database=<db>;Port=<port>;Username=<username>;Password=<password>",

  // Your S3-compatible bucket
  "testStorage": {
    "endpoint": "<your_endpoint>",
    "accessKey": "<your_access_key>",
    "secretKey": "<your_secret_key>",
    "bucket": "<your_bucket>",
    "ssl": true
  }
}
```

运行程序。

```
$ dotnet run
```

## Judger

构建。

```
$ cargo build
```

运行。

```
$ path/to/rurikawa connect <your_coordinator_host>
```
