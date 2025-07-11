// 認可を許可するボタンを表示するページ。
// ボタンをクリックすると、auth-server の認可エンドポイント (/authorize) にリダイレクトされる。

import {
  authServerUrl,
  generateCodeChallenge,
  generateCodeVerifier,
  resourceServerUrl,
} from "~/utils";
import { useStore } from "./store";
import { useEffect } from "react";

// 通常は認可の許可画面を表示する前に認証を行うことが多い
export default function OAuth2() {
  const setItem = useStore((s) => s.setItem);
  const reset = useStore((s) => s.reset);

  const clientId = useStore((s) => s.getItem("oauth2:client_id"));
  const redirectUri = useStore((s) => s.getItem("oauth2:redirect_uri"));
  const accessToken = useStore((s) => s.getItem("oauth2:access_token"));
  const idToken = useStore((s) => s.getItem("openid:id_token"));

  const onClickAuthButton = async () => {
    const codeVerifier = generateCodeVerifier();
    const state = codeVerifier; // CSRF 用のランダム文字列、今回は code_verifier をそのまま使用
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const nonce = crypto.randomUUID();

    // token エンドポイントで必要な値は session storage に保存しておく
    setItem("oauth2:code_verifier", codeVerifier);
    setItem("oauth2:state", state);
    setItem("openid:nonce", nonce);

    const searchParams = new URLSearchParams({
      response_type: "code",
      client_id: clientId!,
      redirect_uri: redirectUri!,
      scope: "openid",
      state: state,
      code_challenge: codeChallenge,
      code_challenge_method: "S256",
      nonce,
    });
    // 認可エンドポイントへリダイレクト
    window.location.href = `${authServerUrl}/authorize?${searchParams.toString()}`;
  };

  const onClickGetResourceButton = async () => {
    const resource = await fetch(`${resourceServerUrl}/resource`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    if (!resource.ok) {
      alert("リソースの取得に失敗しました。");
      return;
    }
    alert(await resource.text());
  };

  useEffect(() => {
    // 5分後に session storage をクリアする
    const id = setInterval(() => {
      reset();
    }, 5 * 60 * 1000);
    return () => clearInterval(id);
  }, [reset]);

  return (
    <div>
      <h1>OAuth2 デモ</h1>
      {!clientId && <a href="/oauth2/clients">OAuth2 Clients の登録</a>}
      {accessToken && (
        <>
          <p>アクセストークン: {accessToken}</p>
          <p>ID token: {idToken}</p>
          <button onClick={onClickGetResourceButton}>リソースを取得</button>
          <button onClick={() => reset()}>リセット</button>
        </>
      )}
      {/* クライアントを登録した後に auth server に認可リクエストを送る */}
      {!accessToken && clientId && redirectUri && (
        <>
          <p>クライアント ID: {clientId}</p>
          <p>リダイレクト URI: {redirectUri}</p>
          <p>認可リクエストを承認してください。</p>
          <button onClick={onClickAuthButton}>承認する</button>
        </>
      )}
    </div>
  );
}
