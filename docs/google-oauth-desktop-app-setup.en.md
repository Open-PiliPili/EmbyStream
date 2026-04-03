# Google OAuth Desktop App Setup

中文版: [简体中文教程](google-oauth-desktop-app-setup.zh-CN.md)

This guide is for first-time users. It shows how to create a Google OAuth
Client of type `Desktop app` from scratch, then use it with
`embystream auth google`.

## What You Will Get

After finishing this guide, you will have:

- a Google Cloud project
- Google Drive API enabled
- an OAuth consent screen
- an OAuth Client of type `Desktop app`
- a `client_id`
- a `client_secret`
- a way to get `access_token` and `refresh_token` with EmbyStream CLI

## Why `Desktop app` Is Required

EmbyStream uses the installed-app OAuth flow with a localhost callback.
If you create the wrong OAuth client type, Google may reject login with:

```text
Error 400: redirect_uri_mismatch
```

The most common reason is using a `Web application` client instead of a
`Desktop app` client.

## Before You Start

Prepare:

- a Google account
- a browser that can sign in to Google
- EmbyStream CLI on your machine

## Step 1: Open Google Cloud Console

Open:

```text
https://console.cloud.google.com/
```

If this is your first time, Google may ask you to accept terms first.

## Step 2: Create a New Project

1. Click the project selector at the top.
2. Click `New Project`.
3. Enter a project name, for example `EmbyStream`.
4. Click `Create`.
5. Wait for Google to finish creating it.
6. Switch to the new project.

## Step 3: Enable Google Drive API

1. In the left menu, open `APIs & Services`.
2. Click `Library`.
3. Search for `Google Drive API`.
4. Click `Google Drive API`.
5. Click `Enable`.

If this step is skipped, your OAuth flow may succeed but Drive requests will
fail later.

## Step 4: Configure the OAuth Consent Screen

1. In the left menu, open `APIs & Services`.
2. Click `OAuth consent screen`.
3. Choose `External` if you are using a normal personal Google account.
4. Click `Create`.

Then fill the basic fields:

- `App name`: for example `EmbyStream`
- `User support email`: choose your email
- `Developer contact information`: enter your email

Then continue and save.

Notes:

- If Google asks for additional fields, keep them simple.
- For personal use, you usually do not need branding beyond the basic fields.

## Step 5: Add Yourself as a Test User

If your app is still in testing mode, only listed test users can log in.

1. Go back to `OAuth consent screen`.
2. Find the `Test users` section.
3. Click `Add users`.
4. Add the Google account you will use for authorization.
5. Save.

If you skip this step, Google may block authorization even if the client is
configured correctly.

## Step 6: Create the Correct OAuth Client

1. In the left menu, open `APIs & Services`.
2. Click `Credentials`.
3. Click `Create Credentials`.
4. Choose `OAuth client ID`.
5. For `Application type`, select `Desktop app`.
6. Enter a name, for example `EmbyStream Desktop`.
7. Click `Create`.

Google will then show:

- `Client ID`
- `Client secret`

Copy both and keep them safe.

## Step 7: Run the EmbyStream CLI

Use:

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET
```

What happens next:

- EmbyStream prints the authorization URL
- EmbyStream tries to open a browser
- you sign in to Google
- you approve readonly Google Drive access
- EmbyStream receives the callback on localhost
- EmbyStream prints:
  - `access_token`
  - `refresh_token`
  - `expires_at`

## Step 8: If the Machine Has No Browser

Use:

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET \
  --no-browser
```

Important:

- `--no-browser` only means EmbyStream will not try to open the browser itself
- it is still the installed-app flow, not Google device flow
- you still need a browser environment that can complete the authorization

## Step 9: Put Tokens into Your Config

After successful authorization, copy the values into your `googleDrive`
backend node.

Typical fields:

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

You can also use `drive_id` instead of `drive_name`.

## Common Errors

### Error 400: `redirect_uri_mismatch`

Cause:

- you created the wrong OAuth client type
- or you are not using a `Desktop app` client

Fix:

- create a new OAuth client
- choose `Desktop app`
- use the new `client_id` and `client_secret`

### Access blocked or app not verified

Cause:

- your OAuth consent screen is incomplete
- or your Google account is not listed as a test user

Fix:

- complete the consent screen fields
- add your Google account to `Test users`

### Google Drive API request fails after login

Cause:

- `Google Drive API` was not enabled in the project

Fix:

- go to `APIs & Services` -> `Library`
- enable `Google Drive API`

## Security Notes

- Keep `client_secret` and `refresh_token` private.
- `access_token` expires, but EmbyStream can refresh it later by using
  `refresh_token`.
- If you use `redirect` mode for `googleDrive`, the token may be exposed to
  the client side. Prefer `proxy` or `accel_redirect` when possible.

## Related Documents

- [CLI usage](cli.md)
- [Configuration reference](configuration-reference.md)
- [User guide](user-guide.md)
