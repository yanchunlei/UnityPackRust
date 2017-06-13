/*
 * This file is part of the UnityPack rust package.
 * (c) Istvan Fehervari <gooksl@gmail.com>
 *
 * All rights reserved 2017
 */

use assetbundle::AssetBundle;
use assetbundle::Signature;
use assetbundle::FSDescriptor;
use binaryreader::Teller;
use std::io;
use std::io::Read;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Seek;
use std::io::SeekFrom;
use byteorder::{BigEndian,ReadBytesExt};
use lzma;

pub struct Asset {
    pub name: String,
    bundle_offset: u64,
}

impl Asset {

    pub fn new(bundle: &mut AssetBundle) -> io::Result<Asset> {
        
        let is_compressed = bundle.is_compressed();
        let ref descriptor = bundle.descriptor;
        
        let decompressed: Vec<u8>;
        
        let mut asset = Asset {
            bundle_offset: 0,
            name: String::new(),
        };  
         
         {
            let mut buffer = match &mut bundle.signature {
                &mut Signature::UnityFS(ref mut buf) => {
                    return Ok(Asset {
                        bundle_offset: buf.tell(),
                        name: String::new(),
                    });
                },
                &mut Signature::UnityWeb(ref mut buf)|
                &mut Signature::UnityRaw(ref mut buf) => {
                    buf
                },
                _ => {return Err(Error::new(ErrorKind::InvalidData, "Cannot load asset from unknown signature"));}
            };

            let offset = buffer.tell();    

            let header_size: u32;
            if !is_compressed {
                asset.name = try!(buffer.read_string());
                header_size = try!(buffer.read_u32::<BigEndian>());
                try!(buffer.read_u32::<BigEndian>());  // size
            } else {
                header_size = match descriptor {
                    &FSDescriptor::Raw(ref desc) => {desc.asset_header_size},
                    _ => {return Err(Error::new(ErrorKind::InvalidData, "Invalid raw descriptor"));},
                };
            }

            let ofs = buffer.tell(); // save current offset so pointer can be later restored
            if is_compressed {
                let mut compressed_data = Vec::new();
                try!(buffer.read_to_end(&mut compressed_data));
                decompressed = match lzma::decompress(&mut compressed_data) {
                    Ok(data) => data,
                    Err(err) => {return Err(Error::new(ErrorKind::InvalidData, format!("{}", err)));},
                };
                asset.bundle_offset = 0;
                try!(buffer.seek(SeekFrom::Start(ofs))); // restore pointer
                
            } else {
                asset.bundle_offset = offset + header_size as u64 - 4;
                if asset.is_resource() {
                    asset.bundle_offset -= asset.name.len() as u64;
                }
                return Ok(asset);
            }
        }
        
        // replace buffer in signature
        bundle.signature = Signature::UnityRawCompressed(decompressed);
        
        Ok(asset)
    }

    pub fn is_resource(&self) -> bool {
        self.name.as_str().ends_with(".resource")
    }

}
