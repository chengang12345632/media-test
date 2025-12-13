import type {
  ApiResponse,
  DeviceInfo,
  RecordingInfo,
  StartPlaybackRequest,
  StartPlaybackResponse,
} from '../types/api'

const API_BASE = '/api/v1'

class ApiClient {
  async get<T>(path: string): Promise<ApiResponse<T>> {
    const response = await fetch(`${API_BASE}${path}`)
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    return response.json()
  }

  async post<T>(path: string, data: any): Promise<ApiResponse<T>> {
    console.log('POST request:', `${API_BASE}${path}`, data)
    const response = await fetch(`${API_BASE}${path}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    })
    console.log('Response status:', response.status, response.statusText)
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    const jsonData = await response.json()
    console.log('Response data:', jsonData)
    return jsonData
  }

  async delete(path: string): Promise<void> {
    const response = await fetch(`${API_BASE}${path}`, {
      method: 'DELETE',
    })
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
  }

  // 设备管理
  async getDevices(): Promise<DeviceInfo[]> {
    const response = await this.get<DeviceInfo[]>('/devices')
    return response.data || []
  }

  async getDevice(deviceId: string): Promise<DeviceInfo> {
    const response = await this.get<DeviceInfo>(`/devices/${deviceId}`)
    if (!response.data) {
      throw new Error('Device not found')
    }
    return response.data
  }

  // 录像管理
  async getRecordings(deviceId: string): Promise<RecordingInfo[]> {
    const response = await this.get<RecordingInfo[]>(
      `/devices/${deviceId}/recordings`
    )
    return response.data || []
  }

  // 播放控制
  async startPlayback(
    fileId: string,
    request: StartPlaybackRequest
  ): Promise<StartPlaybackResponse> {
    const response = await this.post<StartPlaybackResponse>(
      `/playback/start`,
      { ...request, file_id: fileId }
    )
    if (!response.data) {
      throw new Error('Failed to start playback')
    }
    return response.data
  }

  async stopStream(sessionId: string): Promise<void> {
    await this.delete(`/stream/${sessionId}`)
  }
}

export const apiClient = new ApiClient()
