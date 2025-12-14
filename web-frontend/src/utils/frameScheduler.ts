/**
 * FrameScheduler - 精确的帧显示时机控制器
 * 
 * 用于 WebCodecs 播放器，实现精确的帧显示时机控制，
 * 确保视频按照正确的帧率播放，处理过早和过晚到达的帧。
 */

interface FrameQueueItem {
  frame: VideoFrame
  pts: number // Presentation timestamp in microseconds
  arrivalTime: number // When the frame arrived (performance.now())
}

interface FrameStats {
  totalFrames: number
  droppedFrames: number
  displayedFrames: number
  averageDelay: number
}

export class FrameScheduler {
  private targetFps: number
  private baseTime: number | null = null
  private frameQueue: FrameQueueItem[] = []
  private droppedFrames: number = 0
  private displayedFrames: number = 0
  private totalDelay: number = 0
  private scheduledTimeout: number | null = null
  private isRunning: boolean = false
  private displayCallback: ((frame: VideoFrame) => void) | null = null

  // 配置参数
  private readonly EARLY_THRESHOLD_MS = 100 // 提前超过100ms视为过早
  private readonly LATE_THRESHOLD_MS = 50 // 延迟超过50ms视为过晚
  private readonly MAX_QUEUE_SIZE = 10 // 最大队列长度

  constructor(targetFps: number = 30) {
    this.targetFps = targetFps
    console.log(`FrameScheduler initialized with target FPS: ${targetFps}`)
  }

  /**
   * 设置帧显示回调函数
   */
  setDisplayCallback(callback: (frame: VideoFrame) => void): void {
    this.displayCallback = callback
  }

  /**
   * 添加帧到队列
   */
  addFrame(frame: VideoFrame, pts: number): void {
    const arrivalTime = performance.now()

    // 初始化基准时间
    if (this.baseTime === null) {
      this.baseTime = arrivalTime - pts / 1000 // 转换微秒到毫秒
      console.log(`FrameScheduler base time initialized: ${this.baseTime}`)
    }

    // 检查队列是否已满
    if (this.frameQueue.length >= this.MAX_QUEUE_SIZE) {
      console.warn('Frame queue full, dropping oldest frame')
      const oldFrame = this.frameQueue.shift()
      if (oldFrame) {
        oldFrame.frame.close()
        this.droppedFrames++
      }
    }

    // 添加到队列
    this.frameQueue.push({ frame, pts, arrivalTime })

    // 如果调度器未运行，启动它
    if (!this.isRunning) {
      this.isRunning = true
      this.scheduleNextFrame()
    }
  }

  /**
   * 判断帧是否应该显示
   */
  private shouldDisplayFrame(item: FrameQueueItem, currentTime: number): 'display' | 'wait' | 'drop' {
    if (this.baseTime === null) return 'wait'

    const targetDisplayTime = this.baseTime + item.pts / 1000 // 转换微秒到毫秒
    const timeDiff = targetDisplayTime - currentTime

    if (timeDiff > this.EARLY_THRESHOLD_MS) {
      // 过早：需要等待
      return 'wait'
    } else if (timeDiff < -this.LATE_THRESHOLD_MS) {
      // 过晚：丢弃
      return 'drop'
    } else {
      // 在合理范围内：显示
      return 'display'
    }
  }

  /**
   * 计算显示延迟
   */
  private calculateDisplayDelay(item: FrameQueueItem, currentTime: number): number {
    if (this.baseTime === null) return 0

    const targetDisplayTime = this.baseTime + item.pts / 1000
    const delay = Math.max(0, targetDisplayTime - currentTime)
    return delay
  }

  /**
   * 调度下一帧
   */
  private scheduleNextFrame(): void {
    if (this.frameQueue.length === 0) {
      this.isRunning = false
      return
    }

    const currentTime = performance.now()
    const item = this.frameQueue[0]

    const decision = this.shouldDisplayFrame(item, currentTime)

    switch (decision) {
      case 'display':
        // 显示帧
        this.frameQueue.shift()
        this.displayFrame(item)
        // 立即调度下一帧
        this.scheduleNextFrame()
        break

      case 'drop':
        // 丢弃过晚的帧
        console.warn(`Dropping late frame, delay: ${currentTime - (this.baseTime! + item.pts / 1000)}ms`)
        this.frameQueue.shift()
        item.frame.close()
        this.droppedFrames++
        // 立即调度下一帧
        this.scheduleNextFrame()
        break

      case 'wait':
        // 计算等待时间
        const delay = this.calculateDisplayDelay(item, currentTime)
        // 使用 setTimeout 延迟显示
        this.scheduledTimeout = window.setTimeout(() => {
          this.scheduleNextFrame()
        }, delay)
        break
    }
  }

  /**
   * 显示帧
   */
  private displayFrame(item: FrameQueueItem): void {
    if (this.displayCallback) {
      try {
        this.displayCallback(item.frame)
        this.displayedFrames++

        // 记录延迟统计
        const currentTime = performance.now()
        const targetDisplayTime = this.baseTime! + item.pts / 1000
        const actualDelay = Math.abs(currentTime - targetDisplayTime)
        this.totalDelay += actualDelay
      } catch (err) {
        console.error('Error displaying frame:', err)
      }
    }

    // 关闭帧以释放资源
    item.frame.close()
  }

  /**
   * 获取统计信息
   */
  getStats(): FrameStats {
    const totalFrames = this.displayedFrames + this.droppedFrames
    const averageDelay = totalFrames > 0 ? this.totalDelay / this.displayedFrames : 0

    return {
      totalFrames,
      droppedFrames: this.droppedFrames,
      displayedFrames: this.displayedFrames,
      averageDelay
    }
  }

  /**
   * 更新目标帧率
   */
  setTargetFps(fps: number): void {
    this.targetFps = fps
    console.log(`FrameScheduler target FPS updated to: ${fps}`)
  }

  /**
   * 重置调度器
   */
  reset(): void {
    // 清理队列中的所有帧
    for (const item of this.frameQueue) {
      item.frame.close()
    }
    this.frameQueue = []

    // 清理定时器
    if (this.scheduledTimeout !== null) {
      clearTimeout(this.scheduledTimeout)
      this.scheduledTimeout = null
    }

    // 重置状态
    this.baseTime = null
    this.droppedFrames = 0
    this.displayedFrames = 0
    this.totalDelay = 0
    this.isRunning = false

    console.log('FrameScheduler reset')
  }

  /**
   * 清理资源
   */
  destroy(): void {
    this.reset()
    this.displayCallback = null
  }
}
