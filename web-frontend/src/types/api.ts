export interface ApiResponse<T> {
  status: 'success' | 'error'
  data?: T
  error?: string
}

export interface DeviceInfo {
  device_id: string
  device_name: string
  device_type: 'camera' | 'recorder' | 'simulator' | 'gateway'
  connection_status: 'online' | 'offline' | 'reconnecting'
  connection_time: string
  last_heartbeat: string
  capabilities: DeviceCapabilities
}

export interface DeviceCapabilities {
  max_resolution: string
  supported_formats: string[]
  max_bitrate: number
  supports_playback_control: boolean
  supports_recording: boolean
}

export interface RecordingInfo {
  file_id: string
  device_id: string
  file_name: string
  file_path: string
  file_size: number
  duration: number
  format: string
  resolution: string
  bitrate: number
  frame_rate: number
  created_time: string
  modified_time: string
}

export interface StartPlaybackRequest {
  client_id: string
  start_position?: number
}

export interface StartPlaybackResponse {
  session_id: string
  playback_url: string
}
