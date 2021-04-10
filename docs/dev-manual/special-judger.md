# SPJ（Special Judge）说明

当基础的评测系统不能满足要求时，可以使用 SPJ（Special Judge，特殊评测系统）来扩展评测系统的功能。此外，当需要从评测结果获取得分时，必须使用 SPJ 进行分析。

## 运行环境

SPJ 应当是一个合法的 JavaScript ES2020 脚本。这个脚本将被 [QuickJS][] 解释执行。

SPJ 脚本的内存占用上限是 50MiB。

SPJ 的运行环境支持以下 JS 特性：

- BigInt/Float/Decimal
- Date
- JSON
- RegExp
- Promise
- TypedArrays
- Eval

不支持以下 JS 特性：

- ES6 Modules (换句话说，所有功能必须都包含在这一个文件内)

## API

作为 SPJ 使用的脚本应当声明以下全局函数（以 TypeScript 格式声明）：

```ts
// 可选，初始化整个 SPJ，在所有样例运行前调用
function specialJudgeInit(config: JudgerPublicConfig): Promise | void;
// 可选，在执行前修改所有样例的执行步骤，可能会被调用多次
function specialJudgeTransformExec(exec: Step[]): Promise<Step[]> | Step[];
// 可选，初始化单个样例，在执行样例中操作之前调用
function specialJudgeCaseInit(case: Case, mapping: Map<string, string>): Promise | void;

// 可选，在样例执行完毕后调用。在普通模式下，返回 `1` 是 AC，`-1` 是 WA。
//
// 如果开启了 SPJ 评分模式，则还可以返回任意正数作为分值，基准分 1 分。此时 `true` 代表 1 分。
// 本题的实际得分是 返回值 * 该题分值。
function specialJudgeCase(results: StepResult[]): Promise<SpjResult> | SpjResult;

// 将要执行的指令
interface Step {
    command: string;
    isUserCommand: boolean;
}
// 执行单个指令的结果
interface StepResult {
    command: string;
    isUserCommand: boolean;
    stdout: string;
    stderr: string;
    return_code: number;
}
// 单个测试样例
interface Case {
    name: string;
    expectedOut?: string;
    should_fail: boolean;
}
// 测试结果
interface SpjResult {
    // 是否为 AC
    accepted: boolean,
    // 分值
    score?: number,
    // 错误原因
    reason?: string,
    // 与正确输出对比的结果
    diff: string
}
```

此外，SPJ 会将以下函数导出到全局命名空间：

```ts
// 读取题目文件夹下的指定文件，作为字符串返回。`path` 以题目文件夹根目录为基础，
// 不能延伸到题目文件夹以外。如果读取出现错误，则会返回 undefined。
function readFile(path: string): Promise<string | undefined>;

// 控制台
const console: Console

// 向评测机的日志里输出记录
interface Console {
    // 输出日志，等同于 `console.info()`
    log(val: string);
    // 以 DEBUG 等级输出日志
    debug(val: string);
    // 以 INFO  等级输出日志
    info(val: string);
    // 以 WARN  等级输出日志
    warn(val: string);
    // 以 ERROR 等级输出日志
    error(val: string);
}
```

## 示例

以下是一个超级简单的从标准错误流（stderr）获取运行时间并计算得分的 SPJ：

```js
function specialJudgeCaseCommand(case, results) {
    if(results.length === 0) return false;
    let myLastResult = results[results.length - 1];
    let time = parseFloat(myLastResult.stderr);
    // 满分是 0.3 秒
    return time / 0.3;
}
```
