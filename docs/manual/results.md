# 评测结果说明

## 评测阶段

| 阶段名称   | 含义                               |
| ---------- | ---------------------------------- |
| Queued     | 正在排队                           |
| Dispatched | 已经发送到评测机，但是还没开始评测 |
| Fetching   | 评测机正在下载你的代码             |
| Compiling  | 正在构建（评测机暂不支持）         |
| Running    | 正在运行评测程序                   |
| Finished   | 评测已经结束                       |
| Cancelled  | 由于手动或服务端问题，任务被取消   |
| Skipped    | 任务被跳过（未使用）               |

## 任务评测结果

| 结果名称      | 含义                                             |
| ------------- | ------------------------------------------------ |
| Accepted      | 评测过程中没有出现问题（可能有失败的样例）       |
| CompileError  | 编译你的程序时出现错误                           |
| PipelineError | （未使用）                                       |
| JudgerError   | 评测机出现了内部错误                             |
| Aborted       | 任务被取消                                       |
| OtherError    | 出现了不能识别的其他错误（通常是评测机出问题了） |

## 单个样例评测结果

| 缩写 | 结果名称            | 含义                                 |
| ---- | ------------------- | ------------------------------------ |
| AC   | Accepted            | 结果正确，撒花                       |
| WA   | WrongAnswer         | 成功运行，但是结果有误               |
| RE   | RuntimeError        | 运行时出现了错误                     |
| PF   | PipelineFailed      | 运行时有某一步输出不为 0             |
| TLE  | TimeLimitExceeded   | 超时了                               |
| MLE  | MemoryLimitExceeded | 占用内存过大                         |
| NR   | NotRun              | 没有运行                             |
| OE   | OtherError          | 出现了其他错误（通常是评测机的问题） |
