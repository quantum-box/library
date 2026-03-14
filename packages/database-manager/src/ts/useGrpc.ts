import { useMemo } from "react";
import { PromiseClient, createPromiseClient } from "@bufbuild/connect";
import { createGrpcWebTransport } from "@bufbuild/connect-web";
import { DatabaseManager } from "./gen/v1/database_manager_connect";

const baseUrl =
  process.env.NODE_ENV === "production"
    ? "https://sfa-service.tachy.one"
    : "http://localhost:50051";

const webTransport = createGrpcWebTransport({ baseUrl });

export function useGrpc(tenantId: string): {
  grpc: PromiseClient<typeof DatabaseManager>;
  headers: { "x-tenant-id": string };
} {
  const grpc = useMemo(
    () => createPromiseClient(DatabaseManager, webTransport),
    []
  );

  return {
    grpc,
    headers: {
      "x-tenant-id": tenantId,
    }
  };
}
