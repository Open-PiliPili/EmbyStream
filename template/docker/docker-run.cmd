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
  embystream

命令解释:
-d: 后台运行容器
--name: 设置容器名称，如果环境变量 CONTAINER_NAME 未设置，则默认为 embystream
-p: 映射端口
-e: 设置环境变量 (时区, 用户ID, 组ID, 文件权限掩码)
-v: 挂载卷，将您的本地配置文件映射到容器内
--privileged: 给予容器特权模式
--log-opt: 设置日志驱动和选项
--restart: 设置容器的重启策略
embystream: 运行的镜像名称