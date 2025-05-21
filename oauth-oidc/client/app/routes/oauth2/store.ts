import { create } from "zustand";

type StoreKey =
  | "oauth2:state"
  | "oauth2:client_id"
  | "oauth2:redirect_uri"
  | "oauth2:code_verifier"
  | "oauth2:access_token";

type Store = {
  data: Record<string, string>;
  setItem: (key: StoreKey, value: string) => void;
  getItem: (key: StoreKey) => string | undefined;
  reset: () => void;
};

export const useStore = create<Store>((set, get) => ({
  data: {},
  setItem: (key, value) => {
    set((state) => ({
      data: {
        ...state.data,
        [key]: value,
      },
    }));
    sessionStorage.setItem(key, value);
  },
  getItem: (key) => {
    const value = get().data[key];
    return value ?? sessionStorage.getItem(key) ?? undefined;
  },
  reset: () => {
    set({ data: {} });
    sessionStorage.clear();
  },
}));
