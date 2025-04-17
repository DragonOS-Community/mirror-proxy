# 第一阶段：构建阶段
FROM rust:1.84 as builder

# 创建工作目录
WORKDIR /usr/src/mirror-proxy

# 复制项目文件
COPY . .

# 构建发布版本
RUN cargo build --release

# 第二阶段：运行阶段
FROM ubuntu:22.04

# 安装必要的运行时依赖
RUN apt-get update && \
    apt-get install -y libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY ./templates ./templates

# 从构建阶段复制二进制文件
COPY --from=builder /usr/src/mirror-proxy/target/release/mirror-proxy /app/mirror-proxy

# 复制配置文件
COPY config.toml /app/config.toml

# 暴露端口(根据项目实际端口配置)
EXPOSE 8080

# 设置启动命令
CMD ["/app/mirror-proxy"]
