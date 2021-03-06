use bytes::{Buf, BufMut};
use protobuf::reflect::MessageDescriptor;
use protobuf::{CodedInputStream, CodedOutputStream};
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::Status;

use crate::grpc;

#[derive(Debug, Clone)]
pub struct ProtobufCodec {
    request: MessageDescriptor,
    response: MessageDescriptor,
}

pub struct ProtobufEncoder {
    descriptor: MessageDescriptor,
}

pub struct ProtobufDecoder {
    descriptor: MessageDescriptor,
}

impl ProtobufCodec {
    pub fn new(request: MessageDescriptor, response: MessageDescriptor) -> Self {
        ProtobufCodec { request, response }
    }
}

impl Default for ProtobufCodec {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Codec for ProtobufCodec {
    type Encode = <ProtobufEncoder as Encoder>::Item;
    type Decode = <ProtobufDecoder as Decoder>::Item;

    type Encoder = ProtobufEncoder;
    type Decoder = ProtobufDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        ProtobufEncoder {
            descriptor: self.request.clone(),
        }
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtobufDecoder {
            descriptor: self.response.clone(),
        }
    }
}

impl Encoder for ProtobufEncoder {
    type Item = grpc::Request;
    type Error = Status;

    fn encode(&mut self, mut item: Self::Item, dst: &mut EncodeBuf) -> Result<(), Self::Error> {
        debug_assert_eq!(&item.body().descriptor_dyn(), &self.descriptor);
        item.body_mut()
            .write_to_dyn(&mut CodedOutputStream::new(&mut dst.writer()))
            .map_err(|err| {
                tracing::error!("{}", err);
                tonic::Status::internal(err.to_string())
            })?;
        Ok(())
    }
}

impl Decoder for ProtobufDecoder {
    type Item = grpc::Response;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf) -> Result<Option<Self::Item>, Self::Error> {
        let mut item = self.descriptor.new_instance();
        item.merge_from_dyn(&mut CodedInputStream::new(&mut src.reader()))
            .map_err(|err| {
                tracing::error!("{}", err);
                tonic::Status::internal(err.to_string())
            })?;
        Ok(Some(grpc::Response::new(item)))
    }
}
