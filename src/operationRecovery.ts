import type { BrewOperation } from "./types";

export function hasMissingCaskAppSource(operation: BrewOperation) {
  return operation.status === "failed" &&
    operation.target?.kind === "cask" &&
    operation.logs.some((log) => /App source ['"].+\.app['"] is not there/i.test(log.line));
}

export function operationFailureGuidance(operation: BrewOperation) {
  const logs = operation.logs.map((log) => log.line).join("\n");
  if (/sudo:.*(?:terminal|password)|a password is required|no tty present/i.test(logs)) {
    return "安装需要终端中的管理员认证。请在当前任务结束后复制命令到终端执行；BrewDesk 不收集密码。";
  }
  if (/operation not permitted|tcc denied|kTCCServiceSystemPolicyAppBundles|permission denied/i.test(logs)) {
    return "日志包含访问被拒绝。请核对系统弹窗及拒绝的路径；应用管理权限与管理员密码是不同的授权，尚不能仅凭此日志判断原因。";
  }
  if (operation.status === "failed" && operation.action === "upgrade-all") {
    return "批量更新未全部完成，部分项目可能已更新。请刷新可更新列表，再对剩余项目逐项重试。";
  }
  if (operation.status === "running" && /(?:Error:.*(?:Failed to download|Download failed)|curl: \(\d+\))/i.test(logs)) {
    return "已有下载报错，Homebrew 仍可能继续处理其他项目；当前任务尚未全部成功，请查看末尾日志。";
  }
  return null;
}
