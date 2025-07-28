docker run -d \
  --name ${CONTAINER_NAME:-embystream} \
  -p 50001:50001 \
  -e TZ="Asia/Shanghai" \
  -e PUID=1000 \
  -e PGID=1000 \
  -e UMASK=022 \
  -v ./config/config.toml:/config/embystream/config.toml \
  --privileged \
  --log-driver json-file \
  --log-opt max-size=50m \
  --log-opt max-file=3 \
  --restart unless-stopped \
  openpilipili/embystream:latest

Command Explanation:
-d: Run the container in detached (background) mode
--name: Set the container name; if the environment variable CONTAINER_NAME is not set, defaults to 'embystream'
-p: Map ports
-e: Set environment variables (timezone, user ID, group ID, file permission mask)
-v: Mount volume; map your local configuration file into the container
--privileged: Grant the container privileged mode
--log-opt: Configure log driver and its options
--restart: Set the container's restart policy
embystream: The name of the image to run