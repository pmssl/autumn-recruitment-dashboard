interface Window {
  desktop?: {
    load(): Promise<unknown[] | null>;
    save(data: unknown[]): Promise<void>;
  };
}
