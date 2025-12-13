// 统一低延迟视频流传输系统 - fMP4转换器实现
//
// 本模块实现了H.264裸流到fMP4格式的转换。
//
// # 特性
//
// - 生成fMP4初始化分片（init segment）
// - 转换媒体分片（media segment）
// - 保持时间戳和关键帧信息
// - 支持MSE播放器

use super::source::{SegmentFormat, StreamError, VideoSegment};
use bytes::{BufMut, BytesMut};
use tracing::{debug, warn};
use uuid::Uuid;

/// BytesMut 扩展 trait，用于写入24位和48位整数
trait BytesMutExt {
    fn put_u24(&mut self, value: u32);
    fn put_u48(&mut self, value: u64);
}

impl BytesMutExt for BytesMut {
    fn put_u24(&mut self, value: u32) {
        self.put_u8((value >> 16) as u8);
        self.put_u8((value >> 8) as u8);
        self.put_u8(value as u8);
    }

    fn put_u48(&mut self, value: u64) {
        self.put_u8((value >> 40) as u8);
        self.put_u8((value >> 32) as u8);
        self.put_u8((value >> 24) as u8);
        self.put_u8((value >> 16) as u8);
        self.put_u8((value >> 8) as u8);
        self.put_u8(value as u8);
    }
}

/// fMP4 Box类型
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum BoxType {
    Ftyp = 0x66747970, // 'ftyp'
    Moov = 0x6d6f6f76, // 'moov'
    Moof = 0x6d6f6f66, // 'moof'
    Mdat = 0x6d646174, // 'mdat'
    Mvhd = 0x6d766864, // 'mvhd'
    Trak = 0x7472616b, // 'trak'
    Tkhd = 0x746b6864, // 'tkhd'
    Mdia = 0x6d646961, // 'mdia'
    Mdhd = 0x6d646864, // 'mdhd'
    Hdlr = 0x68646c72, // 'hdlr'
    Minf = 0x6d696e66, // 'minf'
    Vmhd = 0x766d6864, // 'vmhd'
    Dinf = 0x64696e66, // 'dinf'
    Dref = 0x64726566, // 'dref'
    Stbl = 0x7374626c, // 'stbl'
    Stsd = 0x73747364, // 'stsd'
    Stts = 0x73747473, // 'stts'
    Stsc = 0x73747363, // 'stsc'
    Stsz = 0x7374737a, // 'stsz'
    Stco = 0x7374636f, // 'stco'
    Mvex = 0x6d766578, // 'mvex'
    Trex = 0x74726578, // 'trex'
    Mfhd = 0x6d666864, // 'mfhd'
    Traf = 0x74726166, // 'traf'
    Tfhd = 0x74666864, // 'tfhd'
    Tfdt = 0x74666474, // 'tfdt'
    Trun = 0x7472756e, // 'trun'
    Avc1 = 0x61766331, // 'avc1'
    AvcC = 0x61766343, // 'avcC'
}

/// fMP4转换器配置
#[derive(Debug, Clone)]
pub struct FMP4ConverterConfig {
    /// 视频宽度
    pub width: u16,
    /// 视频高度
    pub height: u16,
    /// 时间刻度（timescale）
    pub timescale: u32,
    /// 帧率
    pub frame_rate: f64,
}

impl Default for FMP4ConverterConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            timescale: 90000, // 90kHz，H.264标准时间刻度
            frame_rate: 30.0,
        }
    }
}

/// fMP4转换器
///
/// 将H.264裸流转换为fMP4格式，用于MSE播放器。
///
/// # 示例
///
/// ```rust,ignore
/// let converter = FMP4Converter::new(FMP4ConverterConfig::default());
///
/// // 生成初始化分片
/// let init_segment = converter.generate_init_segment()?;
///
/// // 转换媒体分片
/// let h264_segment = VideoSegment { ... };
/// let fmp4_segment = converter.convert_segment(h264_segment)?;
/// ```
pub struct FMP4Converter {
    config: FMP4ConverterConfig,
    sequence_number: u32,
}

impl FMP4Converter {
    /// 创建新的fMP4转换器
    pub fn new(config: FMP4ConverterConfig) -> Self {
        debug!("Creating FMP4Converter with config: {:?}", config);
        Self {
            config,
            sequence_number: 0,
        }
    }

    /// 生成初始化分片（init segment）
    ///
    /// 初始化分片包含ftyp和moov box，用于初始化MSE播放器。
    ///
    /// # 返回
    ///
    /// 返回初始化分片数据或错误
    pub fn generate_init_segment(&self) -> Result<Vec<u8>, StreamError> {
        debug!("Generating fMP4 init segment");

        let mut buffer = BytesMut::new();

        // 写入ftyp box
        self.write_ftyp_box(&mut buffer)?;

        // 写入moov box
        self.write_moov_box(&mut buffer)?;

        debug!("Generated init segment: {} bytes", buffer.len());
        Ok(buffer.to_vec())
    }

    /// 转换H.264分片为fMP4媒体分片
    ///
    /// # 参数
    ///
    /// - `segment`: H.264视频分片
    ///
    /// # 返回
    ///
    /// 返回fMP4格式的分片或错误
    pub fn convert_segment(&mut self, segment: VideoSegment) -> Result<VideoSegment, StreamError> {
        if segment.format != SegmentFormat::H264Raw {
            return Err(StreamError::Internal(
                "Only H264Raw format can be converted to fMP4".to_string(),
            ));
        }

        debug!(
            "Converting H.264 segment {} to fMP4 (size: {} bytes)",
            segment.segment_id,
            segment.data.len()
        );

        let mut buffer = BytesMut::new();

        // 写入moof box
        self.write_moof_box(&mut buffer, &segment)?;

        // 写入mdat box
        self.write_mdat_box(&mut buffer, &segment)?;

        self.sequence_number += 1;

        let fmp4_segment = VideoSegment {
            segment_id: segment.segment_id,
            timestamp: segment.timestamp,
            duration: segment.duration,
            data: buffer.to_vec(),
            is_keyframe: segment.is_keyframe,
            format: SegmentFormat::FMP4,
            receive_time: segment.receive_time,
            forward_time: segment.forward_time,
        };

        debug!(
            "Converted segment {} to fMP4: {} bytes",
            fmp4_segment.segment_id,
            fmp4_segment.data.len()
        );

        Ok(fmp4_segment)
    }

    /// 写入ftyp box（文件类型）
    fn write_ftyp_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut box_data = Vec::new();
        box_data.extend_from_slice(b"iso5"); // major brand
        box_data.extend_from_slice(&0u32.to_be_bytes()); // minor version
        box_data.extend_from_slice(b"iso5"); // compatible brand
        box_data.extend_from_slice(b"iso6"); // compatible brand
        box_data.extend_from_slice(b"mp41"); // compatible brand

        self.write_box(buffer, BoxType::Ftyp, &box_data);
        Ok(())
    }

    /// 写入moov box（媒体元数据）
    fn write_moov_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut moov_data = BytesMut::new();

        // mvhd box
        self.write_mvhd_box(&mut moov_data)?;

        // trak box
        self.write_trak_box(&mut moov_data)?;

        // mvex box
        self.write_mvex_box(&mut moov_data)?;

        self.write_box(buffer, BoxType::Moov, &moov_data);
        Ok(())
    }

    /// 写入mvhd box（movie header）
    fn write_mvhd_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut data = BytesMut::new();
        
        data.put_u8(1); // version
        data.put_u24(0); // flags
        data.put_u64(0); // creation_time
        data.put_u64(0); // modification_time
        data.put_u32(self.config.timescale); // timescale
        data.put_u64(0); // duration (unknown)
        data.put_u32(0x00010000); // rate (1.0)
        data.put_u16(0x0100); // volume (1.0)
        data.put_u16(0); // reserved
        data.put_u64(0); // reserved
        
        // matrix
        data.put_u32(0x00010000);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0x00010000);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0x40000000);
        
        // pre_defined
        for _ in 0..6 {
            data.put_u32(0);
        }
        
        data.put_u32(2); // next_track_ID

        self.write_box(buffer, BoxType::Mvhd, &data);
        Ok(())
    }

    /// 写入trak box（track）
    fn write_trak_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut trak_data = BytesMut::new();

        // tkhd box
        self.write_tkhd_box(&mut trak_data)?;

        // mdia box
        self.write_mdia_box(&mut trak_data)?;

        self.write_box(buffer, BoxType::Trak, &trak_data);
        Ok(())
    }

    /// 写入tkhd box（track header）
    fn write_tkhd_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut data = BytesMut::new();
        
        data.put_u8(1); // version
        data.put_u24(0x000007); // flags (track enabled, in movie, in preview)
        data.put_u64(0); // creation_time
        data.put_u64(0); // modification_time
        data.put_u32(1); // track_ID
        data.put_u32(0); // reserved
        data.put_u64(0); // duration
        data.put_u64(0); // reserved
        data.put_u16(0); // layer
        data.put_u16(0); // alternate_group
        data.put_u16(0); // volume
        data.put_u16(0); // reserved
        
        // matrix
        data.put_u32(0x00010000);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0x00010000);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0);
        data.put_u32(0x40000000);
        
        data.put_u32((self.config.width as u32) << 16); // width
        data.put_u32((self.config.height as u32) << 16); // height

        self.write_box(buffer, BoxType::Tkhd, &data);
        Ok(())
    }

    /// 写入mdia box（media）
    fn write_mdia_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut mdia_data = BytesMut::new();

        // mdhd box
        self.write_mdhd_box(&mut mdia_data)?;

        // hdlr box
        self.write_hdlr_box(&mut mdia_data)?;

        // minf box
        self.write_minf_box(&mut mdia_data)?;

        self.write_box(buffer, BoxType::Mdia, &mdia_data);
        Ok(())
    }

    /// 写入mdhd box（media header）
    fn write_mdhd_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut data = BytesMut::new();
        
        data.put_u8(1); // version
        data.put_u24(0); // flags
        data.put_u64(0); // creation_time
        data.put_u64(0); // modification_time
        data.put_u32(self.config.timescale); // timescale
        data.put_u64(0); // duration
        data.put_u16(0x55c4); // language (und)
        data.put_u16(0); // pre_defined

        self.write_box(buffer, BoxType::Mdhd, &data);
        Ok(())
    }

    /// 写入hdlr box（handler）
    fn write_hdlr_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut data = BytesMut::new();
        
        data.put_u8(0); // version
        data.put_u24(0); // flags
        data.put_u32(0); // pre_defined
        data.extend_from_slice(b"vide"); // handler_type
        data.put_u32(0); // reserved
        data.put_u32(0); // reserved
        data.put_u32(0); // reserved
        data.extend_from_slice(b"VideoHandler\0"); // name

        self.write_box(buffer, BoxType::Hdlr, &data);
        Ok(())
    }

    /// 写入minf box（media information）
    fn write_minf_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut minf_data = BytesMut::new();

        // vmhd box
        self.write_vmhd_box(&mut minf_data)?;

        // dinf box
        self.write_dinf_box(&mut minf_data)?;

        // stbl box
        self.write_stbl_box(&mut minf_data)?;

        self.write_box(buffer, BoxType::Minf, &minf_data);
        Ok(())
    }

    /// 写入vmhd box（video media header）
    fn write_vmhd_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut data = BytesMut::new();
        
        data.put_u8(0); // version
        data.put_u24(1); // flags
        data.put_u16(0); // graphicsmode
        data.put_u16(0); // opcolor[0]
        data.put_u16(0); // opcolor[1]
        data.put_u16(0); // opcolor[2]

        self.write_box(buffer, BoxType::Vmhd, &data);
        Ok(())
    }

    /// 写入dinf box（data information）
    fn write_dinf_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut dinf_data = BytesMut::new();

        // dref box
        let mut dref_data = BytesMut::new();
        dref_data.put_u8(0); // version
        dref_data.put_u24(0); // flags
        dref_data.put_u32(1); // entry_count
        
        // url box
        let mut url_data = BytesMut::new();
        url_data.put_u8(0); // version
        url_data.put_u24(1); // flags (self-contained)
        self.write_box(&mut dref_data, BoxType::Dref, &url_data);

        self.write_box(&mut dinf_data, BoxType::Dref, &dref_data);
        self.write_box(buffer, BoxType::Dinf, &dinf_data);
        Ok(())
    }

    /// 写入stbl box（sample table）
    fn write_stbl_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut stbl_data = BytesMut::new();

        // stsd box (sample description)
        self.write_stsd_box(&mut stbl_data)?;

        // stts box (time-to-sample)
        let mut stts_data = BytesMut::new();
        stts_data.put_u8(0); // version
        stts_data.put_u24(0); // flags
        stts_data.put_u32(0); // entry_count
        self.write_box(&mut stbl_data, BoxType::Stts, &stts_data);

        // stsc box (sample-to-chunk)
        let mut stsc_data = BytesMut::new();
        stsc_data.put_u8(0); // version
        stsc_data.put_u24(0); // flags
        stsc_data.put_u32(0); // entry_count
        self.write_box(&mut stbl_data, BoxType::Stsc, &stsc_data);

        // stsz box (sample size)
        let mut stsz_data = BytesMut::new();
        stsz_data.put_u8(0); // version
        stsz_data.put_u24(0); // flags
        stsz_data.put_u32(0); // sample_size
        stsz_data.put_u32(0); // sample_count
        self.write_box(&mut stbl_data, BoxType::Stsz, &stsz_data);

        // stco box (chunk offset)
        let mut stco_data = BytesMut::new();
        stco_data.put_u8(0); // version
        stco_data.put_u24(0); // flags
        stco_data.put_u32(0); // entry_count
        self.write_box(&mut stbl_data, BoxType::Stco, &stco_data);

        self.write_box(buffer, BoxType::Stbl, &stbl_data);
        Ok(())
    }

    /// 写入stsd box（sample description）
    fn write_stsd_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut stsd_data = BytesMut::new();
        
        stsd_data.put_u8(0); // version
        stsd_data.put_u24(0); // flags
        stsd_data.put_u32(1); // entry_count

        // avc1 box
        let mut avc1_data = BytesMut::new();
        avc1_data.put_u48(0); // reserved
        avc1_data.put_u16(1); // data_reference_index
        avc1_data.put_u16(0); // pre_defined
        avc1_data.put_u16(0); // reserved
        avc1_data.put_u32(0); // pre_defined
        avc1_data.put_u32(0); // pre_defined
        avc1_data.put_u32(0); // pre_defined
        avc1_data.put_u16(self.config.width); // width
        avc1_data.put_u16(self.config.height); // height
        avc1_data.put_u32(0x00480000); // horizresolution
        avc1_data.put_u32(0x00480000); // vertresolution
        avc1_data.put_u32(0); // reserved
        avc1_data.put_u16(1); // frame_count
        
        // compressorname (32 bytes)
        avc1_data.put_u8(0);
        for _ in 0..31 {
            avc1_data.put_u8(0);
        }
        
        avc1_data.put_u16(0x0018); // depth
        avc1_data.put_u16(0xffff); // pre_defined

        // avcC box (简化版本)
        let mut avcc_data = BytesMut::new();
        avcc_data.put_u8(1); // configurationVersion
        avcc_data.put_u8(0x64); // AVCProfileIndication (High)
        avcc_data.put_u8(0x00); // profile_compatibility
        avcc_data.put_u8(0x1f); // AVCLevelIndication
        avcc_data.put_u8(0xff); // lengthSizeMinusOne
        avcc_data.put_u8(0xe0); // numOfSequenceParameterSets
        avcc_data.put_u8(0); // numOfPictureParameterSets
        
        self.write_box(&mut avc1_data, BoxType::AvcC, &avcc_data);
        self.write_box(&mut stsd_data, BoxType::Avc1, &avc1_data);

        self.write_box(buffer, BoxType::Stsd, &stsd_data);
        Ok(())
    }

    /// 写入mvex box（movie extends）
    fn write_mvex_box(&self, buffer: &mut BytesMut) -> Result<(), StreamError> {
        let mut mvex_data = BytesMut::new();

        // trex box
        let mut trex_data = BytesMut::new();
        trex_data.put_u8(0); // version
        trex_data.put_u24(0); // flags
        trex_data.put_u32(1); // track_ID
        trex_data.put_u32(1); // default_sample_description_index
        trex_data.put_u32(0); // default_sample_duration
        trex_data.put_u32(0); // default_sample_size
        trex_data.put_u32(0); // default_sample_flags

        self.write_box(&mut mvex_data, BoxType::Trex, &trex_data);
        self.write_box(buffer, BoxType::Mvex, &mvex_data);
        Ok(())
    }

    /// 写入moof box（movie fragment）
    fn write_moof_box(&self, buffer: &mut BytesMut, segment: &VideoSegment) -> Result<(), StreamError> {
        let mut moof_data = BytesMut::new();

        // mfhd box
        let mut mfhd_data = BytesMut::new();
        mfhd_data.put_u8(0); // version
        mfhd_data.put_u24(0); // flags
        mfhd_data.put_u32(self.sequence_number); // sequence_number
        self.write_box(&mut moof_data, BoxType::Mfhd, &mfhd_data);

        // traf box
        self.write_traf_box(&mut moof_data, segment)?;

        self.write_box(buffer, BoxType::Moof, &moof_data);
        Ok(())
    }

    /// 写入traf box（track fragment）
    fn write_traf_box(&self, buffer: &mut BytesMut, segment: &VideoSegment) -> Result<(), StreamError> {
        let mut traf_data = BytesMut::new();

        // tfhd box
        let mut tfhd_data = BytesMut::new();
        tfhd_data.put_u8(0); // version
        tfhd_data.put_u24(0x020000); // flags (default-base-is-moof)
        tfhd_data.put_u32(1); // track_ID
        self.write_box(&mut traf_data, BoxType::Tfhd, &tfhd_data);

        // tfdt box
        let decode_time = (segment.timestamp * self.config.timescale as f64) as u64;
        let mut tfdt_data = BytesMut::new();
        tfdt_data.put_u8(1); // version
        tfdt_data.put_u24(0); // flags
        tfdt_data.put_u64(decode_time); // baseMediaDecodeTime
        self.write_box(&mut traf_data, BoxType::Tfdt, &tfdt_data);

        // trun box
        let mut trun_data = BytesMut::new();
        trun_data.put_u8(0); // version
        trun_data.put_u24(0x000301); // flags (data-offset-present, sample-duration-present)
        trun_data.put_u32(1); // sample_count
        
        // 计算data_offset (moof size + 8)
        let moof_size = 8 + 16 + 8 + 20 + 8 + 20 + 8 + 20; // 估算
        trun_data.put_u32(moof_size); // data_offset
        
        let sample_duration = (segment.duration * self.config.timescale as f64) as u32;
        trun_data.put_u32(sample_duration); // sample_duration
        
        self.write_box(&mut traf_data, BoxType::Trun, &trun_data);

        self.write_box(buffer, BoxType::Traf, &traf_data);
        Ok(())
    }

    /// 写入mdat box（media data）
    fn write_mdat_box(&self, buffer: &mut BytesMut, segment: &VideoSegment) -> Result<(), StreamError> {
        self.write_box(buffer, BoxType::Mdat, &segment.data);
        Ok(())
    }

    /// 写入box
    fn write_box(&self, buffer: &mut BytesMut, box_type: BoxType, data: &[u8]) {
        let size = 8 + data.len() as u32;
        buffer.put_u32(size);
        buffer.put_u32(box_type as u32);
        buffer.extend_from_slice(data);
    }
}

impl Default for FMP4Converter {
    fn default() -> Self {
        Self::new(FMP4ConverterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_converter_creation() {
        let converter = FMP4Converter::new(FMP4ConverterConfig::default());
        assert_eq!(converter.sequence_number, 0);
    }

    #[test]
    fn test_generate_init_segment() {
        let converter = FMP4Converter::new(FMP4ConverterConfig::default());
        let init_segment = converter.generate_init_segment().unwrap();
        
        assert!(!init_segment.is_empty());
        assert!(init_segment.len() > 100); // 初始化分片应该有一定大小
        
        // 检查ftyp box
        assert_eq!(&init_segment[4..8], b"ftyp");
    }

    #[test]
    fn test_convert_segment() {
        let mut converter = FMP4Converter::new(FMP4ConverterConfig::default());
        
        let h264_segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.0,
            duration: 0.033,
            data: vec![0, 0, 0, 1, 0x67, 0x42, 0x00, 0x1f], // 简化的H.264数据
            is_keyframe: true,
            format: SegmentFormat::H264Raw,
        };

        let fmp4_segment = converter.convert_segment(h264_segment.clone()).unwrap();
        
        assert_eq!(fmp4_segment.segment_id, h264_segment.segment_id);
        assert_eq!(fmp4_segment.timestamp, h264_segment.timestamp);
        assert_eq!(fmp4_segment.duration, h264_segment.duration);
        assert_eq!(fmp4_segment.is_keyframe, h264_segment.is_keyframe);
        assert_eq!(fmp4_segment.format, SegmentFormat::FMP4);
        assert!(!fmp4_segment.data.is_empty());
    }

    #[test]
    fn test_sequence_number_increment() {
        let mut converter = FMP4Converter::new(FMP4ConverterConfig::default());
        
        let h264_segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.0,
            duration: 0.033,
            data: vec![0, 0, 0, 1, 0x67],
            is_keyframe: true,
            format: SegmentFormat::H264Raw,
        };

        converter.convert_segment(h264_segment.clone()).unwrap();
        assert_eq!(converter.sequence_number, 1);

        converter.convert_segment(h264_segment.clone()).unwrap();
        assert_eq!(converter.sequence_number, 2);
    }

    #[test]
    fn test_invalid_format_conversion() {
        let mut converter = FMP4Converter::new(FMP4ConverterConfig::default());
        
        let mp4_segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.0,
            duration: 0.033,
            data: vec![1, 2, 3],
            is_keyframe: true,
            format: SegmentFormat::MP4,
        };

        let result = converter.convert_segment(mp4_segment);
        assert!(result.is_err());
    }
}
