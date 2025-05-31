# OAuth と OpenID Connect を実装して理解する

- [OAuth 2.0 の認可サーバを自作してみたった](https://zenn.dev/hitoe_kami/articles/0050-articles-go-oauth2-server)
- [フルスクラッチして理解するOpenID Connect](https://www.m3tech.blog/entry/2024/03/05/150000)

## ドキュメント

フロントエンドの立ち上げ。

```sh
cd client && npm run dev
```

サーバーの立ち上げ。

```sh
docker compose up

cd auth-server && cargo watch -x run
cd resource-server && cargo watch -x run
```
