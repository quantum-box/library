import { DurableObject } from 'cloudflare:workers'

export interface Env {
  PHOTON_SYNC_ROOMS: DurableObjectNamespace<PhotonSyncRoom>
  PHOTON_CLOUD_ENGINE_BASE_URL?: string
  PHOTON_EDGE_SERVICE_TOKEN?: string
}

export class PhotonSyncRoom extends DurableObject<Env> {
  async fetch(): Promise<Response> {
    return new Response('upstream', { status: 200 })
  }

  async webSocketMessage(sender: WebSocket, message: string | ArrayBuffer | ArrayBufferView): Promise<void> {
    const context = this.ctx as unknown as {
      upstreamMessages?: Array<{ sender: WebSocket; message: string | ArrayBuffer | ArrayBufferView }>
    }
    context.upstreamMessages?.push({ sender, message })
    return undefined
  }
}

export default {
  async fetch(): Promise<Response> {
    return new Response('upstream', { status: 200 })
  },
}
