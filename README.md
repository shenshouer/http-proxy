# http-proxy

基于pingora的反向代理，支持动态添加代理的域名并自动解析域名获取IP作为后端，支持健康检查。

管理API使用pingora ServerApp框架，路由层面使用axum router。


## 使用

```shell
RUST_LOG=info cargo r

```

1. 添加代理

```shell
curl -H "Content-Type: application/json" -i -d '{"domain": "www.google.com"}' 'http://localhost:6100/domain'
curl -H "Content-Type: application/json" -i -d '{"domain": "chatgpt.com"}' 'http://localhost:6100/domain'
```

2. 查询代理

```shell
% curl 'http://localhost:6100/domain' | jq .
{
  "www.google.com": [
    "172.217.194.99:443",
    "172.217.194.103:443",
    "172.217.194.104:443",
    "172.217.194.105:443",
    "172.217.194.106:443",
    "172.217.194.147:443"
  ],
  "chatgpt.com": [
    "104.18.32.47:443",
    "172.64.155.209:443"
  ]
}
```

3. 删除代理

```shell
curl -XDELETE -H "Content-Type: application/json" -i -d '{"domain": "www.google.com"}' 'http://localhost:6100/domain'
curl -XDELETE -H "Content-Type: application/json" -i -d '{"domain": "chatgpt.com"}' 'http://localhost:6100/domain'
```

4. 通过代理访问

```shell
curl -H "Host: www.google.com" http://localhost:6188
```

## 计划

- [x] 动态添加代理
- [x] 动态自动保存cookie
- [x] 支持添加代理规则
