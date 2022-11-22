<h1 align="center">mastodon-notifier</h1>
<div align="center">
  <strong>
    Lightweight mastodon desktop notification daemon
  </strong>
</div>

<br />


# Introduction

Mastodon notifier it's a simple rust application that connects to the user notification stream
and send desktop notification.

# Installation

With cargo

``` sh
cargo install mastodon-notifier
```

# Connecting to Mastodon's API


`mastodon-notifier` needs to access the the Mastodon API of your instance.

Follow this instruction for configuring a `mastodon-notifier`:

- First create an app on your instance at url `https://<instance_url>/settings/applications` with `read` scope checked
and redirect uri `urn:ietf:wg:oauth:2.0:oob`. Once it's created you will see `Client Key` and `Client Secret` 

- Run the `mastodon notifier` for configuring the account with `mastodon-notifier --host <host> --user <user> --mode config`
  and enter the `Client Key` and the `Client Secret`. You will be redirected then to your instance for authorization.
  Once authorized your instance will show an authorization code that you need to provide to the configurator.
  The configuration is now complete and `mastodon-notifier` will save the token in the user keyring.


# Usage

The binary is named `mastodon-notifier` and can run as daemon which listens to user notification stream
and sends desktop notifications.

``` sh
❯ ./target/release/mastodon-notifier --host hachyderm.io --user wolf4ood --mode daemon
2022-11-22T18:49:01.532345Z  INFO mastodon_notifier::daemon: Found stored token for user wolf4ood@hachyderm.io
2022-11-22T18:49:02.063228Z  INFO mastodon_notifier::daemon: Started mastodon notify daemon on account wolf4ood@hachyderm.io
```

The daemon look in the keyring for the token associated to the account `username@instance` configured in the previous step.

If it's not available the daemon will wait until it's ready
