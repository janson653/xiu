pub mod errors;
pub mod gop;
pub mod metadata;

use {
    self::gop::Gops,
    crate::channels::define::ChannelData,
    bytes::BytesMut,
    errors::CacheError,
    gop::Gop,
    std::collections::VecDeque,
    xflv::{define, demuxer_tag},
};
#[derive(Clone)]
pub struct Cache {
    metadata: metadata::MetaData,
    metadata_timestamp: u32,
    video_seq: BytesMut,
    video_timestamp: u32,
    audio_seq: BytesMut,
    audio_timestamp: u32,
    gops: Gops,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Cache {
    pub fn new(gop_num: usize) -> Self {
        Self {
            metadata: metadata::MetaData::new(),
            metadata_timestamp: 0,
            video_seq: BytesMut::new(),
            video_timestamp: 0,
            audio_seq: BytesMut::new(),
            audio_timestamp: 0,
            gops: Gops::new(gop_num),
        }
    }

    //, values: Vec<Amf0ValueType>
    pub fn save_metadata(&mut self, chunk_body: BytesMut, timestamp: u32) {
        self.metadata.save(chunk_body);
        self.metadata_timestamp = timestamp;
    }

    pub fn get_metadata(&self) -> Option<ChannelData> {
        let data = self.metadata.get_chunk_body();
        if !data.is_empty() {
            Some(ChannelData::MetaData {
                timestamp: self.metadata_timestamp,
                data,
            })
        } else {
            None
        }
    }

    pub fn save_audio_seq(
        &mut self,
        chunk_body: BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = demuxer_tag::AudioTagHeaderDemuxer::new(chunk_body.clone());
        let tag = parser.parse_tag_header()?;

        let channel_data = ChannelData::Audio {
            timestamp,
            data: chunk_body.clone(),
        };
        self.gops.save_frame_data(channel_data, false);

        if tag.sound_format == define::sound_format::AAC
            && tag.aac_packet_type == define::aac_packet_type::AAC_SEQHDR
        {
            self.audio_seq = chunk_body;
            self.audio_timestamp = timestamp;
        }

        Ok(())
    }

    pub fn get_audio_seq(&self) -> Option<ChannelData> {
        if !self.audio_seq.is_empty() {
            return Some(ChannelData::Audio {
                timestamp: self.audio_timestamp,
                data: self.audio_seq.clone(),
            });
        }
        None
    }

    pub fn get_video_seq(&self) -> Option<ChannelData> {
        if !self.video_seq.is_empty() {
            return Some(ChannelData::Video {
                timestamp: self.video_timestamp,
                data: self.video_seq.clone(),
            });
        }
        None
    }

    pub fn save_video_seq(
        &mut self,
        chunk_body: BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = demuxer_tag::VideoTagHeaderDemuxer::new(chunk_body.clone());
        let tag = parser.parse_tag_header()?;

        let channel_data = ChannelData::Video {
            timestamp,
            data: chunk_body.clone(),
        };
        let is_key_frame = tag.frame_type == define::frame_type::KEY_FRAME;
        self.gops.save_frame_data(channel_data, is_key_frame);

        if is_key_frame && tag.avc_packet_type == define::avc_packet_type::AVC_SEQHDR {
            self.video_seq = chunk_body;
            self.video_timestamp = timestamp;
        }

        Ok(())
    }

    pub fn get_gops_data(self) -> Option<VecDeque<Gop>> {
        if self.gops.setted() {
            Some(self.gops.get_gops())
        } else {
            None
        }
    }
}
