# Rurikawa

![Rurikawa Header pic](res/header-pic.png)

Rurikawa is a simple Docker-based online judge system for complex multi-file projects with changing building scripts and multi-stage assignments.

Rurikawa（琉璃川）是一个简易的自动评测系统，主要面向构建流程多变、复杂的多文件项目，以及多阶段的作业。

## Features

- Online judging system
- Git repository submission (plus, it's the only way to submit, so you always have your submission versioned)
- Customize assignment building scripts using Dockerfile
- Standalone judgers (judger tags coming soon™)
- ~~"Special judge" scripts for complex judging dynamic scoring~~ Not yet available in v0.5.0
- Horizontally-scalable backend design

### Not-yet implemented

- I18n (Currently only supports Chinese)

## Building 

Building coordinator requires the following tools:

- DotNet 5 SDK

Building web requires the following tools:

- NodeJS v12+
- Yarn 1.22+

Building judger requires the following tools:

- Rust 1.56+
- GCC or Clang that support C11

## Running

### Before running

You'll need these tools to run a Rurikawa server (coordinator + web):

- A [`PostgreSQL`][postgres]-compatible database (You can try [`CockroachDB`][cockroach], though it hasn't been tested).
- A Amazon S3-compatible object storage service (I would recommend [`Minio`][minio] if you're serving your own files).
- Redis.
- Any recent version of `git` inside coordinator environment.

You can refer to the development docker compose file (`dev.docker-compose.yml`) for an example.

You'll need these tools to run a Rurikawa judger:

- A Unix-family operating system (Sadly, windows doesn't work for now).
- Any recent version of `git`.
- Any recent version of `docker`, with API exposed at the default path.
  - You might need to log into a paid account if your clients use many different kinds of build environments - Docker now limits access rates for unpaid accounts.
  - You might need to run [`docuum`][docuum] to manage Docker's build image cache.

[postgres]: https://postgresql.org/
[cockroach]: https://cockroachlabs.com/
[minio]: https://min.io/
[docuum]: https://github.com/stepchowfun/docuum

You can check out the corresponding dockerfiles provided for detailed building instructions.

### Backend

Configure `coordinator/appsettings.json` before running:

```json
{
  // These parts are for controlling logging behavior. Change if you need.
  "Logging": {
    "LogLevel": {
      "Default": "Information",
      "Microsoft": "Warning",
      "Microsoft.Hosting.Lifetime": "Information"
    }
  },

  // Enter information about your database here:
  "pgsqlLink": "Host=<host>;Database=<db>;Port=<port>;Username=<username>;Password=<password>",

  // Enter your Redis host here:
  "redisLink": "<redis>",

  // Enter information about your OSS here:
  "testStorage": {
    "endpoint": "<your_endpoint>",
    "accessKey": "<your_access_key>",
    "secretKey": "<your_secret_key>",
    "bucket": "<your_bucket>",
    "ssl": true
  }
}
```

You can find an example of this file at `coordinator/appsettings.dev.json`.

You'll also need an ECDSA private key in PFX format next to the coordinator CWD, as `certs/dev.pfx` (Subject to change), in order to sign JWTs.

To run coordinator, run:

```
$ dotnet run
```

### Judger

You'll need coordinator running before running judger.

To run the judger the first time, you'll need a register token from coordinator. Visit `<your_web_host>/admin/judger` to get a token, and then run:

```
$ path/to/rurikawa connect <your_coordinator_host> --register-token <token>
```

In subsequent runs, only `rurikawa connect` is needed if the configuration stays the same.

Data created by the judger will be stored at `~/.rurikawa`.

## License

MIT.

Copyright (c) 2020--2021, Karenia Works (Rynco Maekawa & Rami3L Li).

## Naming

Rurikawa is the name of a fictional high school (_Rurikawa High School_ or _Liulichuan High School_, 琉璃川高等学校), in [volvacea][]'s doujin novel series [_Two Centimeters Above The Clouds_][2cm], inside the world of the web novel series [_Illumine Lingao_][lgqm]. A rough guess of this high school's position in real world is near Toupu Village, Longhua District, Haikou, Hainan.

[volvacea]: https://lgqm.gq/space-uid-2378.html
[2cm]: https://lgqm.gq/thread-4190-1-1.html
[lgqm]: https://en.wikipedia.org/wiki/Illumine_Lingao
