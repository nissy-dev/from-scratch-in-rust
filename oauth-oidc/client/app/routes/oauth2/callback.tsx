// 認可サーバーから受け取った認可コードをもとにトークンエンドポイントにリクエストを送信する。
// 認可リクエストを送るときの redirect_uri にはここの path を指定する。

import { use, useEffect, useState } from "react";
import { authServerUrl } from "~/utils";
import { useStore } from "./store";

export default function Callback() {
  const [error, setError] = useState<string>("");
  const setItem = useStore((s) => s.setItem);
  const storedState = useStore((s) => s.getItem("oauth2:state"));
  const clientId = useStore((s) => s.getItem("oauth2:client_id"));
  const redirectUri = useStore((s) => s.getItem("oauth2:redirect_uri"));
  const codeVerifier = useStore((s) => s.getItem("oauth2:code_verifier"));

  const fetchAccessToken = async ({ code }: { code: string }) => {
    if (!clientId || !redirectUri || !codeVerifier) {
      setError("Invalid state");
      return;
    }

    const response = await fetch(`${authServerUrl}/token`, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        grant_type: "authorization_code",
        client_id: clientId,
        redirect_uri: redirectUri,
        code,
        code_verifier: codeVerifier,
      }),
    });
    if (!response.ok) {
      throw new Error("Failed to fetch access token");
    }
    const data = (await response.json()) as {
      access_token: string;
      token_type: string;
      expires_in: number;
      scope: string;
    };
    setItem("oauth2:access_token", data.access_token);
    // トークン取得後に任意のページにリダイレクトする
    window.location.href = "/oauth2";
  };

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const code = params.get("code") as string;
    const returnedState = params.get("state");

    // CSRF 対策
    if (returnedState !== storedState) {
      setError("CSRF attack detected");
      return;
    }

    // トークンエンドポイントにリクエストしてアクセストークンを取得する
    // 取得したアクセストークンは localstorage に保存し、次のページへ redirect する
    fetchAccessToken({ code: code });
  }, []);

  if (error) {
    return <div>{error}</div>;
  }

  return (
    <div>This is callback page! Client is requesting token endpoint...</div>
  );
}
