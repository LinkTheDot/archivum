Archivum is an app focused on building a data warehouse for the site Twitch.tv.
When configured it will track the chats of given channels and store all messages, 
donations, livestreams, bans and more to a central database.

The current limit of channels that can be tracked right now is 5, but this will
be expanded in future updates.

# Setup
First you need to start by generating Twitch access and clien tokens with the right 
permissions. I suggest using [`swiftyspiffy's website for this`](https://twitchtokengenerator.com/).

As an individual the only permission you'll need is `chat:read`.
If you're the owner of a channel, or have moderator permissions, `bits:read` and `channel:read:subscriptions`
would be recommended for accuracy.

Next you'll want to make sure there's a running instance of MySql for the app to access.
This can be running on the hardware, through Docker, in a Kubernetes cluster, or any other way you might
run a Rust binary.

# Creating the Config
Once you have your access and client token, you'll want to create the config file. 
The default path for config files is `./config_files/config.yml`. 
This path can be configured with the `CONFIG_PATH` environment variable.

Inside this file you'll want to setup your values. It'll look something like this:

```yml
twitchNickname: YourLoginNameHere
accessToken: YourAccessTokenHere
clientId: YourClientIdHere
databaseUsername: YourMySqlDatabaseUsernameHere
sqlUserPassword: YourSqlUserPasswordHere
databaseHostAddress: localhost:3306 # This is the default value.
channels: ["ChannelOneNameHere", "ChannelTwoNameHere"] # Up to 5 are allowed for now.

# Optional
logLevel: Info # Set your actual desired level.
database: twitch_tracker_db # This is the default name.
pastebinApiKey: YourPastebinApiKeyHere # Required to use the report generator app.
```

Most values (including secrets) can use the environment to define them instead.
The available list of environment variables is as such:

`TWITCH_ACCESS_TOKEN`, `TWITCH_CLIENT_ID`, `DATABASE_USERNAME`, `DATABASE_HOST_ADDRESS`,
`DATABASE_PASSWORD`, and `PASTEBIN_API_KEY` 

# Running
Once you've setup the config and MySql, you can run the tracker in one of three ways.
- Binary
- Dockerfile
- Kubernetes

## Binary
Running the binary is as simple as building the target like so:
```bash
cargo run --release -p twitch_chat_tracker
```
And grabbing/running the binary from `./target/release/twitch_chat_tracker`
If you setup your config properly it should connect to Twitch's servers and start
writting any incoming data to the database.

## Dockerfile
The docker image for the tracker can be built with the following command:
```bash
docker build --progress=plain -t twitch-chat-logger:latest -f twitch_chat_tracker/Dockerfile .;
```
With any optional environment variables passed in as `-e VARIABLE="value"`.
Then run:
```
docker run --rm twitch-chat-logging:latest
```
To actually run the image. 

If you setup your config properly it should connect to Twitch's servers and start
writting any incoming data to the database.

## Kubernetes
There's already an available deployment file under `twitch_chat_tracker/kube` that
has most things already preconfigured. You just have to put in your own desired values.

Changing:
- `spec.template.spec.containers.image` to pull your correct Docker image.
- CONFIG_PATH under `spec.template.spec.env`
- DATABASE_HOST_ADDRESS under `spec.template.spec.env` and;
- TRACKED_CHANNELS under `spec.template.spec.env`.

I suggest also creating a secret in your cluster for this.
The file should look something like so:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: twitch-chat-logger
type: Opaque
stringData:
  twitchAccessToken: "YourTwitchAccessTokenHere"
  twitchClientId: "YourTwitchClientTokenHere"
  pastebinApiKey: "YourPastebinApiKeyHere"
```
This will plug in to the expected values in the existing deployment file.

If you setup your config and cluster properly it should connect to Twitch's servers and start
writting any incoming data to the database.
