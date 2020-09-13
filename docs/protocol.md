# Rurikawa Judger Interface

Rurikawa Judger Interface (RJI) 是 Rurikawa 项目下的评测机（Judger）和后端（Coordinator）通讯所使用的接口。接口使用 HTTP(S) 和 (Secure) WebSocket 进行连接和信息交换。

接口中的所有信息传递均使用 JSON 格式编码。其中表示消息类型的字段（discriminator）必须被序列化为消息类型的第一个字段，键为 `_t`。

## WebSocket 接口

### 信息模型

#### Coordinator 发出的消息

```ts
/** 分配一个新的任务 */
interface NewJobMsg {
    job: Job,
}

interface Job {
    // TODO: Implement
}
```

#### Judger 发出的消息

```ts
/** 汇报任务的评测进度 */
interface JobProgressMsg {
    id: string,
    stage: JobStage,
    totalPoints?: number,
    finishedPoints: number,
}

/** 汇报任务的结果 */
interface JobResultMsg {
    // TODO: implement
}

/** 汇报评测机的状态 */
interface ClientStatusMsg {
    activeTaskCount: number,
    canAcceptNewTask: boolean,
}
```

