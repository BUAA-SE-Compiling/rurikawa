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

不支持以下 JS 特性：

- eval
- ES6 Modules (换句话说，所有功能必须都包含在这一个文件内)

## API

作为 SPJ 使用的脚本应当声明以下全局函数（以 TypeScript 格式声明）：

```ts
// 可选，初始化整个 SPJ，在所有样例运行前调用
function specialJudgeInit(config: JudgerPublicConfig);
// 可选，在执行前修改执行步骤
function specialJudgeTransformExec(exec: Step[]): Step[];
// 可选，初始化单个样例，在执行样例中操作之前调用
function specialJudgeCaseInit(case: string, mapping: Map<string, string>);

// 必选，分析执行结果，返回 `true` 是 AC，`false` 是 WA。
//
// 如果开启了 SPJ 评分模式，则还可以返回一个数字作为分值，基准分 1 分。此时 `true` 代表 1 分。
// 本题的实际得分是 返回值 * 该题分值。
function specialJudgeCaseCommand(case: string, results: StepResult[]): boolean | number

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
}
```

此外，SPJ 会将以下函数导出到全局命名空间：

```ts
// 读取题目文件夹下的指定文件，作为字符串返回。
function readFile(path: string): Promise<string>;
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
