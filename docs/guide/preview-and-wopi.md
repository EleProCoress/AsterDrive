# 在线预览与 WOPI

::: tip 这一篇覆盖什么
这一篇讲怎么给文件增加额外打开方式，尤其是把 Office 文件交给 OnlyOffice、Collabora 或其他 WOPI 兼容服务打开。普通文本编辑看 [文件编辑](/guide/editing)。
:::

## 先分清三类打开方式

| 类型 | 适合什么 | 关键要求 |
| --- | --- | --- |
| 内置预览器 | 图片、PDF、文本、视频等浏览器能处理的类型 | AsterDrive 自己提供 |
| URL 模板预览器 | 把文件预览链接交给外部网页 | 外部网页能访问 AsterDrive 生成的预览链接 |
| WOPI 打开方式 | Office 文件在线预览或编辑 | WOPI 服务能回连 AsterDrive 的 WOPI API |

WOPI 最常见的用途是让 `docx`、`xlsx`、`pptx` 这类文件在 OnlyOffice 或 Collabora 里打开。AsterDrive 负责文件访问、会话、令牌和锁；真正的 Office 编辑界面由外部 WOPI 服务提供。

## 推荐接入顺序

```text
准备真实 HTTPS 入口
  |
  +-- 填公开站点地址
  |
  +-- 部署或确认 WOPI 服务
  |
  +-- 在预览应用里导入 WOPI Discovery
  |
  +-- 启用对应文件类型的打开方式
  |
  +-- 用真实 Office 文件试开和保存
```

## 入口在哪里

管理员入口：

```text
管理 -> 系统设置 -> 站点配置 -> 预览应用
```

相关配置：

```text
管理 -> 系统设置 -> 站点配置 -> 公开站点地址
管理 -> 系统设置 -> 网络访问
```

反向代理相关说明见 [反向代理](/deployment/reverse-proxy#wopi--office-回调的额外要求)。

## `公开站点地址` 为什么最重要

WOPI 服务打开文件时，不是浏览器自己直接读本地文件。它会拿 AsterDrive 给出的 WOPI URL 回连 AsterDrive，读取文件信息和文件内容。

所以 `公开站点地址` 必须满足：

- 是真实 HTTP(S) 来源
- 推荐正式环境使用 HTTPS
- WOPI 服务所在环境能访问
- 多域名部署时，每个真实入口都逐项添加

例如：

```text
https://drive.example.com
https://office-drive.example.com
```

每一项只填来源层，不要带路径：

```text
https://drive.example.com
```

不要写成：

```text
https://drive.example.com/api
```

## Docker 网络里的常见写法

如果 AsterDrive 和 OnlyOffice / Collabora 都在 Docker 里，有两条常见路线：

| 路线 | 公开站点地址 | 适合场景 |
| --- | --- | --- |
| 都走公网域名 | `https://drive.example.com` | 反向代理和证书已经准备好 |
| 内网域名回连 | `https://drive.internal.example.com` | WOPI 服务在内网，能解析内网域名 |

不要直接把 `公开站点地址` 填成 `http://localhost:3000`。  
对 WOPI 服务来说，`localhost` 通常指它自己的容器或主机，不是 AsterDrive。

## 通过 WOPI Discovery 导入

如果你的 WOPI 服务提供 discovery 地址，推荐用导入方式创建打开方式。

常见形态类似：

```text
https://office.example.com/hosting/discovery
```

导入后，AsterDrive 会根据 discovery 返回的应用信息生成对应打开方式。你仍然需要确认：

- 对应文件扩展名已经启用
- 打开方式排序符合预期
- 打开方式是预览还是编辑
- 弹窗或新标签页打开方式符合你的站点使用习惯

如果 WOPI 服务更新了 discovery 内容，可能需要重新导入或等待 discovery 缓存过期。缓存时长在系统设置的 WOPI 相关配置里。

## URL 模板预览器什么时候用

URL 模板预览器适合“把一个可访问的文件预览链接交给外部网页”。

它和 WOPI 的区别是：

- URL 模板通常不负责保存回写
- 外部服务多数只拿到一个文件 URL
- 对方必须能访问这个 URL
- 内网地址、`localhost`、纯 HTTP 链接经常会失败

内置的 Microsoft / Google 预览器通常属于这类思路。它们更适合公开可访问的文件预览场景，不适合作为私有部署里的通用编辑方案。

## 保存、历史版本和锁

WOPI 保存回 AsterDrive 时，会按覆盖写入处理：

- 保存成功后生成历史版本
- 文件编辑期间会出现锁
- 其他客户端不能随意覆盖、移动或删除被锁文件
- 锁异常残留时，管理员可以到 `管理 -> 锁` 清理

WOPI 是否支持多人协作，取决于你接入的外部服务。AsterDrive 负责按 WOPI 协议提供文件、令牌、会话和锁，不替外部 Office 服务实现协作编辑界面。

## CORS 什么时候需要改

大多数 WOPI 回连问题不是 CORS，而是 WOPI 服务访问不到 `公开站点地址`。

只有在浏览器控制台明确出现 AsterDrive API 跨域错误时，才去这里调整：

```text
管理 -> 系统设置 -> 网络访问 -> 允许跨域来源
```

如果只是 OnlyOffice / Collabora 后端请求 AsterDrive 失败，优先检查网络、域名、证书和反向代理。

## 上线前验收

用真实 Office 文件测试：

1. `docx` 能打开
2. `xlsx` 能打开
3. `pptx` 能打开
4. 保存后 AsterDrive 里文件内容更新
5. 历史版本里能看到新版本
6. 关闭编辑器后锁会释放
7. 分享页或团队空间里的同类文件行为符合预期

如果你只打算提供预览，不提供编辑，也要确认用户界面里的按钮文案和行为符合预期，避免用户以为能保存。

## 常见问题

### 打开方式没有出现

优先检查：

- 预览应用是否启用
- 文件扩展名是否被这条应用覆盖
- 当前用户是否有权限读取该文件
- 预览应用排序里是否被其他打开方式覆盖

### 打开后白屏

最常见原因是 WOPI 服务回连不到 AsterDrive。检查：

- `公开站点地址` 是否真实可达
- WOPI 服务容器或主机能否解析这个域名
- TLS 证书是否被 WOPI 服务信任
- 反向代理是否透传 `/api/v1/wopi/`

### 能打开但保存失败

优先检查：

- 文件是否仍然存在
- 用户是否仍有写权限
- WOPI access token 是否过期
- WOPI 服务和 AsterDrive 的系统时间是否正确
- `管理 -> 锁` 里是否有异常锁

### 内置 Microsoft / Google 预览器打不开

它们通常需要外部服务能访问文件预览链接。私有内网、`localhost`、未受信任证书或纯 HTTP 部署都可能失败。私有部署需要在线编辑时，更推荐接自己的 WOPI 服务。
