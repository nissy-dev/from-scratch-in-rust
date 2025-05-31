export const authServerUrl = "http://localhost:3123";

export const resourceServerUrl = "http://localhost:6244";

// code_verifier は、43 - 128文字の長さのランダムな文字列である必要があります。
// 使える文字は、英大文字、英小文字、数字、"-"、"."、"_"、"~" です。
// 今回は実装の簡略化のため、英大文字、英小文字、数字のみを使用します。
export function generateCodeVerifier(length: number = 100): string {
  const characters =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let codeVerifier = "";
  for (let i = 0; i < length; i++) {
    codeVerifier += characters.charAt(
      Math.floor(Math.random() * characters.length)
    );
  }
  return codeVerifier;
}

// code_challenge は code_verifier を SHA-256 でハッシュ化し、Base64URL エンコードする
export async function generateCodeChallenge(
  codeVerifier: string
): Promise<string> {
  // SHA-256 ハッシュを生成する
  const encoder = new TextEncoder();
  const data = encoder.encode(codeVerifier);
  const hash = await crypto.subtle.digest("SHA-256", data);
  // array buffer の文字列をテキストに変換して Base64 でエンコードする
  const base64String = btoa(String.fromCharCode(...new Uint8Array(hash)));
  // Base64URL 形式に対応させる
  return base64String.replace(/=/g, "").replace(/\+/g, "-").replace(/\//g, "_");
}
