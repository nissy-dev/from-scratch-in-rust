// OAuth Client を登録するを作成する
// Client Secret は本来はクライアント側に保存することはない

import { useState, type MouseEventHandler } from "react";
import { useStore } from "./store";
import { authServerUrl } from "~/utils";

export default function OAuth2Clients() {
  const [clientSecret, setClientSecret] = useState<string>("");

  const setItem = useStore((s) => s.setItem);
  const clientId = useStore((s) => s.getItem("oauth2:client_id"));
  const redirectUri = useStore((s) => s.getItem("oauth2:redirect_uri"));

  const [inputName, setInputName] = useState<string>("sample-client");
  const [inputRedirectUri, setInputRedirectUri] = useState<string>(
    "http://localhost:5173/oauth2/callback"
  );

  const onClick: MouseEventHandler<HTMLButtonElement> = async (e) => {
    e.preventDefault();

    try {
      const response = await fetch(`${authServerUrl}/clients`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          name: inputName,
          redirect_uri: inputRedirectUri,
        }),
      });

      if (!response.ok) {
        throw new Error(`エラーが発生しました: ${response.statusText}`);
      }

      const data = (await response.json()) as {
        client_id: string;
        redirect_uri: string;
        client_secret: string;
      };

      // 成功したらストアに保存
      setItem("oauth2:client_id", data.client_id);
      setItem("oauth2:redirect_uri", data.redirect_uri);
      // client_secret は本来はクライアント側に保存することはない
      // ここではデモのために表示する
      setClientSecret(data.client_secret);
    } catch (err) {
      console.error(err);
      alert("クライアントの登録に失敗しました。");
    }
  };

  const isClientRegistered = clientId && redirectUri;

  return (
    <div>
      <h1>OAuth2 Client の登録</h1>
      {!isClientRegistered && (
        <div>
          <div>
            <label>
              Name:
              <input
                type="text"
                value={inputName}
                onChange={(e) => setInputName(e.target.value)}
              />
            </label>
          </div>
          <div>
            <label>
              Redirect URI:
              <input
                type="text"
                value={inputRedirectUri}
                size={40}
                onChange={(e) => setInputRedirectUri(e.target.value)}
              />
            </label>
          </div>
          <button onClick={(e) => onClick(e)}>登録</button>
        </div>
      )}
      {isClientRegistered && (
        <div>
          <p>クライアント ID: {clientId}</p>
          <p>リダイレクト URI: {redirectUri}</p>
          <p> Client Credentials Grant のコマンド</p>
          {/* prettier-ignore */}
          <code>
            curl -X POST {authServerUrl}/token
              -H "Authorization: Basic {btoa(`${clientId}:${clientSecret}`)}"
              -H "Content-Type: application/x-www-form-urlencoded"
              -d "grant_type=client_credentials&client_id={clientId}&client_secret={clientSecret}&scope=read"
          </code>
        </div>
      )}
      <a href="/oauth2">OAuth2 デモ画面に戻る</a>
    </div>
  );
}
