# SPJ（Special Judge）说明

当基础的评测系统不能满足要求时，可以使用 SPJ（Special Judge，特殊评测系统）来扩展评测系统的功能。

## API

SPJ 应当是一个合法的 JavaScript 脚本。这个脚本将被 [QuickJS][] 解释执行。

SPJ 脚本的内存占用上限是 100MiB。

作为 SPJ 使用的脚本应当具有以下全局函数：

```ts
function 
```
