# 提交作业的格式与方法

## 一些概念

在开始提交作业之前，你需要先了解几个概念：

- [TOML][] 是一种配置文件的格式。你将使用这个格式编写作业的配置文件。
- [Docker][] 是一个运行应用容器的平台。你的作业将被创建成 Docker 镜像，在容器中运行。
  - Docker Image（镜像）是一种保存应用程序和运行环境的文件，通常构建在 Linux 环境上。
  - [Dockerfile][] 是一种描述镜像创建方式的文件格式。你需要用这个格式描述你的作业程序如何被编译。

[toml]: https://toml.io
[docker]: https://docker.io
[dockerfile]: https://docs.docker.com/engine/reference/builder

## 配置和上传

### 仓库

不同于通常见到的单文件 OJ，这个 OJ **只**接受多文件的、以 git 仓库形式保存的作业。这要求你必须拥有一个 Git 平台的账号，比如 GitHub 或者 GitLab，并在上面创建一个仓库。为了防止其他同学看到你的代码，我们建议你创建*一个*私有（private）仓库。之后我们还会说到如何让评测平台访问到私有仓库。

> 如果你已经为这门课程创建过了仓库，且为其他作业提交过代码，你可以使用 `git checkout --orphan <分支名>` 的方式在仓库内创建一个孤立分支来复用这个存储库。
>
> 关于这个问题的详情可以看 [这个 StackOverflow 回答][orphan-branch]

[orphan-branch]: https://stackoverflow.com/questions/1384325/in-git-is-there-a-simple-way-of-introducing-an-unrelated-branch-to-a-repository#4288660

### 配置

#### `Dockerfile`

Dockerfile 控制着你的代码如何被编译。关于 Dockerfile 的完整说明可以看 [官方文档][dockerfile]，这里只讲最基本的内容。你也可以通过查看 [Docker 的官方教程][docker-intro]

下面是一个简单的 Dockerfile 的例子，我们配合着例子讲解：

```dockerfile
FROM ubuntu:16.04
```

`FROM <镜像>[:<版本>]` 语句指定了当前环境的基础镜像。

在这里，我们使用了 `ubuntu` 镜像带有 `16.04` 标签的版本，也就是 Ubuntu 16.04 系统的环境。镜像的名称和版本可以在 [Docker Hub][dockerhub] 里查到；如果你找不到适合你的镜像，你也可以直接用 `ubuntu` 或者 `alpine` 等 Linux 发行版的镜像并自己安装依赖。

为了减少拉取镜像的次数和镜像的大小，我们建议你使用带有 `slim` 或者 `alpine` (基于 Alpine Linux 环境) 标签的镜像。

```dockerfile
RUN apt update && apt install gcc
```

`RUN <指令>` 语句在镜像环境的终端中执行某个命令。在 Linux 环境下，这个命令一般是在 `sh` 或者 `bash` 中运行的。

这里，我们更新了软件包并安装了 gcc。在没有 `WORKDIR` 语句（看下面）指定的情况下，工作路径是根目录。

```dockerfile
WORKDIR /app/
```

`WORKDIR <路径>` 语句可以切换当前的工作路径，如果路径不存在则会被创建。这里，我们把工作路径设置成了 `/app`，也就是说之后的指令都会在 `/app` 文件夹中执行。

```dockerfile
COPY my-program.c my-other-file.c ./
```

`COPY <文件> [更多文件...] <目标>` 语句将本地环境的文件复制到镜像中，类似 `cp` 命令。你可以使用 `COPY ./* /folder/` 把当前目录下的所有文件都复制过去。如果复制的文件多于一个，目标必须是一个文件夹（以 `/` 结尾）。如果目标路径不存在也会自动被创建。

```dockerfile
RUN gcc my-program.c -o my-program
```

以上就是指示 docker 构建你的程序的 dockerfile 内容。在通常情况下，我们还需要使用 `ENTRYPOINT` 指定运行镜像使用的命令；但是评测机不会直接运行镜像，所以我们就略过不讲了。

在编写完你的 dockerfile 之后，就可以把它保存下来了。一般来说，使用文件名 `Dockerfile` 保存在源代码（或者编译器的配置文件）所在的目录就行。**请注意，评测姬只允许 dockerfile 的位置处在构建环境文件夹之内。**

运行 `docker build .` 就可以使用当前目录和当前目录下的 `Dockerfile` 文件构建出一个包含你的程序的镜像。你可以将 `.` 替换成其他目录，或者使用 `-f <file>` 参数更改使用的 dockerfile。

[docker-intro]: https://docs.docker.com/samples/
[dockerhub]: https://hub.docker.com/

#### `judge.toml`

`judge.toml` 会告诉评测姬你的代码如何运行。

下面是一个典型的 `judge.toml` 文件和各个部分的作用：

```toml
# jobs 是一个哈希表，里面包含你的程序在不同题目环境下会使用的编译和运行策略。
# jobs.pascal_lex 表示 pascal_lex 这个题目下你的配置
[jobs.pascal_lex]

# image 规定了镜像如何构建。
# 我们通常使用 Dockerfile 方法，此时填写 source = "dockerfile"
#   使用 path 指定 dockerfile 的构建目录， "." 表示当前目录。
#   使用 file 指定 dockerfile 相对于构建目录的位置，默认是 "./Dockerfile"。
#   使用 tag 指定构建完成之后镜像的名称。没有实际意义，但是方便报错的时候看。
image = { source = "dockerfile", path = ".", tag = "pascal-lex-example" }

# 我们还支持直接使用已经存在的镜像，此时填写 source = "image"
#   此时，使用 tag 直接指定镜像的名称。
# image = { source = "image", tag = "my-image" }

# run 规定了如何评测你的代码。
# run 是一个字符串数组，每一个字符串是一行在终端中运行的命令。
#
# ‘$’ 开头的参数会在运行之前被替换成评测时的样例文件，比如下面的 '$input'
# 在评测的时候就会被依次替换成 '/tests/1.in'、'/tests/2.in' 等等。你可以自行改变
# 这些参数的位置，来适应你的程序的输入方法。
#
# run 中的每一条命令的工作路径都是你在 dockerfile 中最后一次指定的工作路径。
run = [
  "./target/release/pascal-lexer $input",
]
```

### 提交作业

在提交作业的网页中有两个文本框，分别表示你提交的 git 仓库的 **地址** 和 **分支**。

你需要在表示地址的文本框内填写你的仓库的 **HTTPS 拉取地址**（因为很明显你没有我们评测姬所用的 git 的 ssh 公钥）；在分支文本框内填写 **你想要提交的分支** 或 **完整的 commit SHA**。如果分支文本框留空了，则表示提交存储库的默认分支（比如 master 或者 main）。

点击提交按钮就可以提交作业。

被评测的代码来源是 **你提交的时候指定的分支的最新 commit**。

#### 提交私有仓库

嗯，你不想让代码被别人抄走所以建了一个私有仓库。现在你想要把它交上去。

首先，按照上面的方式填好仓库的地址和分支。

然后，在你的仓库所在的平台申请一个 Access Token（访问令牌）。

> GitHub 的用户可以访问 `Settings > Developer Settings > Personal access tokens`，点击 `Generate new token` 并生成一个有 `repo` 权限的令牌。

> GitLab 的用户可以访问 `Settings (设置) > Access Token (访问令牌)`，生成一个有 `read_repository` 权限的令牌。

在仓库地址的 `https://` 之后加上 `<你的用户名>:<你的令牌>@` 这段文字。

> 比如，我的用户名是 `my-username`，申请了一个内容是 `12345` 的令牌，仓库地址是 `https://my-git.com/my-username/my-repo.git`，那么修改后的地址应该是 `https://my-username:12345@my-git.com/my-username/my-repo.git`。

现在提交就好了。

**我们保证只会读取你提交的那一个仓库的内容，且除了你和管理员以外任何人都看不见你的用户名和令牌。**
