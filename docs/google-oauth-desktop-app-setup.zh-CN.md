# Google OAuth Desktop App 创建教程

English version: [Google OAuth Desktop App Setup](google-oauth-desktop-app-setup.en.md)

这是一份给完全新手准备的教程。你可以跟着它，从 0 到 1 创建一个
Google OAuth `Desktop app` 类型的客户端，然后配合
`embystream auth google` 获取认证信息。

## 你最终会得到什么

完成后，你会拥有：

- 一个 Google Cloud 项目
- 已启用的 Google Drive API
- 一个 OAuth 同意屏幕配置
- 一个 `Desktop app` 类型的 OAuth Client
- 一组 `client_id`
- 一组 `client_secret`
- 通过 EmbyStream CLI 获取 `access_token` 和 `refresh_token` 的方法

## 为什么必须选 `Desktop app`

EmbyStream 当前使用的是 installed-app OAuth 流程，依赖本机 localhost
回调地址完成授权。

如果你创建成别的类型，最常见的报错就是：

```text
错误 400：redirect_uri_mismatch
```

大多数情况下，原因就是你创建成了 `Web application`，而不是
`Desktop app`。

## 开始前准备

你需要准备：

- 一个 Google 账号
- 一个可以正常登录 Google 的浏览器
- 已安装好的 EmbyStream CLI

## 第 1 步：打开 Google Cloud Console

打开下面这个地址：

```text
https://console.cloud.google.com/
```

如果你是第一次使用，Google 可能会先让你同意一些服务条款。

## 第 2 步：创建一个新项目

1. 点击页面顶部的项目选择器。
2. 点击 `New Project`。
3. 输入项目名称，比如 `EmbyStream`。
4. 点击 `Create`。
5. 等待项目创建完成。
6. 切换到这个新项目。

## 第 3 步：启用 Google Drive API

1. 左侧菜单进入 `APIs & Services`。
2. 点击 `Library`。
3. 搜索 `Google Drive API`。
4. 点进去。
5. 点击 `Enable`。

这一步不能跳过。否则即使 OAuth 登录成功，后续访问 Google Drive
接口时也会失败。

## 第 4 步：配置 OAuth consent screen

1. 左侧菜单进入 `APIs & Services`。
2. 点击 `OAuth consent screen`。
3. 如果是普通个人账号，一般选择 `External`。
4. 点击 `Create`。

然后填写基础信息：

- `App name`：比如 `EmbyStream`
- `User support email`：选你自己的邮箱
- `Developer contact information`：填写你的邮箱

然后一路继续并保存即可。

说明：

- 如果 Google 要求你补更多字段，先按最基础的信息填完即可。
- 个人自用场景通常不需要做复杂品牌配置。

## 第 5 步：把自己加入 Test users

如果你的应用还处于测试状态，没有加入测试用户的账号通常无法登录。

1. 回到 `OAuth consent screen` 页面。
2. 找到 `Test users` 区域。
3. 点击 `Add users`。
4. 加入你自己要拿来授权的 Google 账号。
5. 保存。

如果跳过这一步，即使你的 Client 创建正确，也可能会被 Google 拦下。

## 第 6 步：创建正确的 OAuth Client

1. 左侧菜单进入 `APIs & Services`。
2. 点击 `Credentials`。
3. 点击 `Create Credentials`。
4. 选择 `OAuth client ID`。
5. 在 `Application type` 中选择 `Desktop app`。
6. 输入一个名称，比如 `EmbyStream Desktop`。
7. 点击 `Create`。

创建完成后，Google 会展示：

- `Client ID`
- `Client secret`

把这两个值复制出来，后面 CLI 会直接用到。

## 第 7 步：运行 EmbyStream CLI

执行：

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET
```

执行后会发生这些事情：

- EmbyStream 打印授权链接
- EmbyStream 默认尝试自动打开浏览器
- 你在浏览器中登录 Google
- 你同意只读的 Google Drive 权限
- EmbyStream 在本机 localhost 接收回调
- EmbyStream 输出：
  - `access_token`
  - `refresh_token`
  - `expires_at`

## 第 8 步：如果当前机器没有浏览器

执行：

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET \
  --no-browser
```

注意：

- `--no-browser` 的意思只是“不自动帮你打开浏览器”
- 它依然是 installed-app OAuth，不是 device flow
- 你仍然需要在一个能完成授权的浏览器环境中打开那个链接

## 第 9 步：把 token 写回配置

认证成功后，把得到的值写到你的 `googleDrive` 节点里。

典型示例：

```toml
[[BackendNode]]
type = "googleDrive"
node_uuid = "google-drive-node-a"
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"
access_token = "YOUR_ACCESS_TOKEN"
refresh_token = "YOUR_REFRESH_TOKEN"
drive_name = "pilipili"
```

如果你已经知道共享盘 ID，也可以使用 `drive_id`。

## 常见错误

### 错误 400：`redirect_uri_mismatch`

原因：

- 你创建错了 OAuth Client 类型
- 或者你用的不是 `Desktop app`

解决方式：

- 重新创建一个新的 OAuth Client
- `Application type` 选择 `Desktop app`
- 使用新生成的 `client_id` 和 `client_secret`

### Access blocked / app not verified

原因：

- OAuth consent screen 没配完整
- 或者当前 Google 账号没有加入 `Test users`

解决方式：

- 把 consent screen 基础信息补完整
- 把你的 Google 账号加入 `Test users`

### 登录成功了，但后续 Drive API 访问失败

原因：

- 你没有在项目里启用 `Google Drive API`

解决方式：

- 进入 `APIs & Services` -> `Library`
- 启用 `Google Drive API`

## 安全注意事项

- `client_secret` 和 `refresh_token` 都属于敏感信息，不要泄漏。
- `access_token` 会过期，但 EmbyStream 后续可以用 `refresh_token`
  自动刷新。
- 如果 `googleDrive` 使用 `redirect` 模式，token 有暴露给客户端的风险。
  能用 `proxy` 或 `accel_redirect` 时，优先用后两者。

## 相关文档

- [CLI 使用说明](cli.md)
- [配置参考](configuration-reference.md)
- [用户指南](user-guide.md)
