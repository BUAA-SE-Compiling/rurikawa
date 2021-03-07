import { HttpErrorResponse } from '@angular/common/http';

const errorCodeStringify = {
  git_no_such_revision: '找不到给定的存储库、分支或提交',
  revision_fetch_timeout: '提交信息拉取超时',
  no_such_suite: '找不到给定的测试组',
  not_in_active_timespan: '不在提交时间段内',

  not_owner: '用户不是该内容的所有者',

  invalid_grant_type: '无效的授权来源',
  invalid_login_info: '登录信息有误或无效',
  not_enough_login_info: '登陆信息不足',

  username_not_unique: '用户名已被占用',
  invalid_username: '用户名不符合要求',
  already_initialized: '程序已经初始化过了',

  judger_no_such_register_token: '找不到这个评测姬注册口令',
  unspecified_content_length: '没有指定内容长度',
};

const CONNECTION_FAILED = '网络连接失败';

export function errorCodeToDescription(code: string) {
  return errorCodeStringify[code] ?? code;
}

export function errorResponseToDescription(resp: HttpErrorResponse) {
  if (resp.status === 0) {
    return CONNECTION_FAILED;
  }
  let errorCode = resp.statusText;
  let response;
  if (resp.error) {
    try {
      response = JSON.parse(resp.error);
    } catch {}
  }
  if (resp.status >= 300 && response?.err) {
    errorCode = errorCodeToDescription(response.err);
  }
  if (response?.message) {
    errorCode += ': ' + response.message;
  }
  return errorCode;
}
