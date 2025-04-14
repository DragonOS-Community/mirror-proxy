# 使用官方nginx镜像作为基础
FROM nginx:latest

# 复制测试用的nginx配置
COPY nginx-test.conf /etc/nginx/conf.d/default.conf

# 创建测试目录结构
RUN mkdir -p /usr/share/nginx/html/test && \
    mkdir -p /usr/share/nginx/html/dir1/dir2 && \
    echo "test1" > /usr/share/nginx/html/test/file1.txt && \
    echo "test2" > /usr/share/nginx/html/test/file2.txt && \
    echo "nested" > /usr/share/nginx/html/dir1/dir2/file3.txt

# 暴露80端口
EXPOSE 80

# 启动nginx并保持前台运行
CMD ["nginx", "-g", "daemon off;"]
