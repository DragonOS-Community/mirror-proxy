# Nginx 测试环境

## 快速启动 (使用docker-compose)

1. 启动服务：
```bash
docker-compose up -d
```

2. 测试访问：
```bash
curl http://localhost:8080
```
或在浏览器打开 http://localhost:8080

## 文件结构
容器内预创建了以下测试文件：
- /test/file1.txt
- /test/file2.txt
- /dir1/dir2/file3.txt

## 管理命令

停止服务：
```bash
docker-compose down
```

重启服务：
```bash
docker-compose restart
```

重新构建并启动：
```bash
docker-compose up -d --build
```
