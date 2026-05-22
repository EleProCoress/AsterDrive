use std::{
    collections::{HashSet, VecDeque},
    io::{Read, Seek, SeekFrom},
};

use crate::types::ImageMediaMetadata;

use super::{clean_metadata_string, dimensions_area};

const TIFF_TAG_IMAGE_WIDTH: u16 = 0x0100;
const TIFF_TAG_IMAGE_LENGTH: u16 = 0x0101;
const TIFF_TAG_MAKE: u16 = 0x010f;
const TIFF_TAG_MODEL: u16 = 0x0110;
const TIFF_TAG_ORIENTATION: u16 = 0x0112;
const TIFF_TAG_MODIFY_DATE: u16 = 0x0132;
const TIFF_TAG_SOFTWARE: u16 = 0x0131;
const TIFF_TAG_ARTIST: u16 = 0x013b;
const TIFF_TAG_SUB_IFD: u16 = 0x014a;
const TIFF_TAG_COPYRIGHT: u16 = 0x8298;
const TIFF_TAG_EXIF_IFD: u16 = 0x8769;
const TIFF_TAG_GPS_IFD: u16 = 0x8825;
const TIFF_TAG_EXPOSURE_TIME: u16 = 0x829a;
const TIFF_TAG_F_NUMBER: u16 = 0x829d;
const TIFF_TAG_ISO: u16 = 0x8827;
const TIFF_TAG_DATE_TIME_ORIGINAL: u16 = 0x9003;
const TIFF_TAG_CREATE_DATE: u16 = 0x9004;
const TIFF_TAG_OFFSET_TIME: u16 = 0x9010;
const TIFF_TAG_OFFSET_TIME_ORIGINAL: u16 = 0x9011;
const TIFF_TAG_OFFSET_TIME_DIGITIZED: u16 = 0x9012;
const TIFF_TAG_EXPOSURE_BIAS: u16 = 0x9204;
const TIFF_TAG_FLASH: u16 = 0x9209;
const TIFF_TAG_FOCAL_LENGTH: u16 = 0x920a;
const TIFF_TAG_EXIF_IMAGE_WIDTH: u16 = 0xa002;
const TIFF_TAG_EXIF_IMAGE_HEIGHT: u16 = 0xa003;
const TIFF_TAG_FOCAL_LENGTH_35MM: u16 = 0xa405;
const TIFF_TAG_LENS_MAKE: u16 = 0xa433;
const TIFF_TAG_LENS_MODEL: u16 = 0xa434;
const TIFF_GPS_LATITUDE_REF: u16 = 0x0001;
const TIFF_GPS_LATITUDE: u16 = 0x0002;
const TIFF_GPS_LONGITUDE_REF: u16 = 0x0003;
const TIFF_GPS_LONGITUDE: u16 = 0x0004;
const TIFF_GPS_ALTITUDE_REF: u16 = 0x0005;
const TIFF_GPS_ALTITUDE: u16 = 0x0006;
const TIFF_FALLBACK_MAX_DEPTH: usize = 8;
const TIFF_FALLBACK_MAX_OFFSETS_PER_TAG: usize = 32;

pub(super) fn enrich_image_metadata_from_reader<R>(
    mut reader: R,
    metadata: &mut ImageMediaMetadata,
) -> std::io::Result<()>
where
    R: Read + Seek,
{
    let mut marker = [0; 4];
    match reader.read_exact(&mut marker) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
        Err(error) => return Err(error),
    }

    if !matches!(&marker, b"II*\0" | b"MM\0*" | b"II+\0" | b"MM\0+") {
        return Ok(());
    }

    reader.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if let Some(tiff_metadata) = parse_tiff_fallback_metadata(&bytes) {
        tiff_metadata.enrich_image_metadata(metadata);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TiffEndian {
    Little,
    Big,
}

impl TiffEndian {
    fn read_u16(self, bytes: &[u8]) -> Option<u16> {
        let bytes: [u8; 2] = bytes.get(..2)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(bytes),
            Self::Big => u16::from_be_bytes(bytes),
        })
    }

    fn read_i16(self, bytes: &[u8]) -> Option<i16> {
        let bytes: [u8; 2] = bytes.get(..2)?.try_into().ok()?;
        Some(match self {
            Self::Little => i16::from_le_bytes(bytes),
            Self::Big => i16::from_be_bytes(bytes),
        })
    }

    fn read_u32(self, bytes: &[u8]) -> Option<u32> {
        let bytes: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        })
    }

    fn read_i32(self, bytes: &[u8]) -> Option<i32> {
        let bytes: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
        Some(match self {
            Self::Little => i32::from_le_bytes(bytes),
            Self::Big => i32::from_be_bytes(bytes),
        })
    }

    fn read_u64(self, bytes: &[u8]) -> Option<u64> {
        let bytes: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
        Some(match self {
            Self::Little => u64::from_le_bytes(bytes),
            Self::Big => u64::from_be_bytes(bytes),
        })
    }

    fn read_i64(self, bytes: &[u8]) -> Option<i64> {
        let bytes: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
        Some(match self {
            Self::Little => i64::from_le_bytes(bytes),
            Self::Big => i64::from_be_bytes(bytes),
        })
    }

    fn read_f32(self, bytes: &[u8]) -> Option<f32> {
        self.read_u32(bytes).map(f32::from_bits)
    }

    fn read_f64(self, bytes: &[u8]) -> Option<f64> {
        self.read_u64(bytes).map(f64::from_bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TiffIfdKind {
    Main,
    Exif,
    Gps,
    Generic,
}

#[derive(Debug)]
struct TiffIfdTask {
    offset: u64,
    kind: TiffIfdKind,
    depth: usize,
}

#[derive(Debug)]
struct TiffEntry {
    tag: u16,
    value: TiffEntryValue,
}

#[derive(Debug)]
enum TiffEntryValue {
    Bytes(Vec<u8>),
    Ascii(String),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    URational(Vec<(u32, u32)>),
    IRational(Vec<(i32, i32)>),
}

impl TiffEntryValue {
    fn string(&self) -> Option<String> {
        match self {
            Self::Ascii(value) => clean_metadata_string(Some(value)),
            Self::Bytes(values) => std::str::from_utf8(values)
                .ok()
                .and_then(|value| clean_metadata_string(Some(value.trim_end_matches('\0')))),
            _ => None,
        }
    }

    fn first_u8(&self) -> Option<u8> {
        match self {
            Self::Bytes(values) => values.first().copied(),
            Self::U16(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            Self::U32(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            Self::U64(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            Self::I16(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            Self::I32(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            Self::I64(values) => values.first().and_then(|value| u8::try_from(*value).ok()),
            _ => None,
        }
    }

    fn first_u16(&self) -> Option<u16> {
        match self {
            Self::Bytes(values) => values.first().copied().map(u16::from),
            Self::U16(values) => values.first().copied(),
            Self::U32(values) => values.first().and_then(|value| u16::try_from(*value).ok()),
            Self::U64(values) => values.first().and_then(|value| u16::try_from(*value).ok()),
            Self::I16(values) => values.first().and_then(|value| u16::try_from(*value).ok()),
            Self::I32(values) => values.first().and_then(|value| u16::try_from(*value).ok()),
            Self::I64(values) => values.first().and_then(|value| u16::try_from(*value).ok()),
            _ => None,
        }
    }

    fn first_u32(&self) -> Option<u32> {
        match self {
            Self::Bytes(values) => values.first().copied().map(u32::from),
            Self::U16(values) => values.first().copied().map(u32::from),
            Self::U32(values) => values.first().copied(),
            Self::U64(values) => values.first().and_then(|value| u32::try_from(*value).ok()),
            Self::I16(values) => values.first().and_then(|value| u32::try_from(*value).ok()),
            Self::I32(values) => values.first().and_then(|value| u32::try_from(*value).ok()),
            Self::I64(values) => values.first().and_then(|value| u32::try_from(*value).ok()),
            _ => None,
        }
    }

    fn first_f64(&self) -> Option<f64> {
        let value = match self {
            Self::Bytes(values) => f64::from(*values.first()?),
            Self::U16(values) => f64::from(*values.first()?),
            Self::U32(values) => f64::from(*values.first()?),
            Self::U64(values) => f64::from(u32::try_from(*values.first()?).ok()?),
            Self::I16(values) => f64::from(*values.first()?),
            Self::I32(values) => f64::from(*values.first()?),
            Self::I64(values) => f64::from(i32::try_from(*values.first()?).ok()?),
            Self::F32(values) => f64::from(*values.first()?),
            Self::F64(values) => *values.first()?,
            Self::URational(values) => {
                let (numerator, denominator) = *values.first()?;
                rational_to_f64(numerator, denominator)?
            }
            Self::IRational(values) => {
                let (numerator, denominator) = *values.first()?;
                signed_rational_to_f64(numerator, denominator)?
            }
            Self::Ascii(_) => return None,
        };
        value.is_finite().then_some(value)
    }

    fn offsets(&self) -> Vec<u64> {
        match self {
            Self::Bytes(values) => values.iter().copied().map(u64::from).collect(),
            Self::U16(values) => values.iter().copied().map(u64::from).collect(),
            Self::U32(values) => values.iter().copied().map(u64::from).collect(),
            Self::U64(values) => values.clone(),
            Self::I16(values) => values
                .iter()
                .filter_map(|value| u64::try_from(*value).ok())
                .collect(),
            Self::I32(values) => values
                .iter()
                .filter_map(|value| u64::try_from(*value).ok())
                .collect(),
            Self::I64(values) => values
                .iter()
                .filter_map(|value| u64::try_from(*value).ok())
                .collect(),
            _ => Vec::new(),
        }
    }

    fn gps_decimal_degrees(&self) -> Option<f64> {
        let parts: Vec<f64> = match self {
            Self::URational(values) => values
                .iter()
                .filter_map(|(numerator, denominator)| rational_to_f64(*numerator, *denominator))
                .collect(),
            Self::IRational(values) => values
                .iter()
                .filter_map(|(numerator, denominator)| {
                    signed_rational_to_f64(*numerator, *denominator)
                })
                .collect(),
            _ => return None,
        };
        if parts.len() < 3 {
            return None;
        }
        let value = parts[0] + parts[1] / 60.0 + parts[2] / 3600.0;
        value.is_finite().then_some(value)
    }
}

#[derive(Debug, Default)]
struct TiffFallbackMetadata {
    dimensions: Vec<(u32, u32)>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    lens_make: Option<String>,
    lens_model: Option<String>,
    f_number: Option<f64>,
    exposure_time_seconds: Option<f64>,
    iso: Option<u32>,
    exposure_bias_ev: Option<f64>,
    flash_mode: Option<u16>,
    focal_length_mm: Option<f64>,
    focal_length_35mm: Option<u32>,
    date_time_original: Option<String>,
    create_date: Option<String>,
    modify_date: Option<String>,
    offset_time: Option<String>,
    offset_time_original: Option<String>,
    offset_time_digitized: Option<String>,
    orientation: Option<u16>,
    gps_latitude_ref: Option<String>,
    gps_latitude: Option<f64>,
    gps_longitude_ref: Option<String>,
    gps_longitude: Option<f64>,
    gps_altitude_ref: Option<u8>,
    gps_altitude_meters: Option<f64>,
    artist: Option<String>,
    copyright: Option<String>,
    software: Option<String>,
}

impl TiffFallbackMetadata {
    fn enrich_image_metadata(self, metadata: &mut ImageMediaMetadata) {
        let taken_at = self.taken_at();
        let gps_latitude = self.signed_gps_latitude();
        let gps_longitude = self.signed_gps_longitude();
        let gps_altitude_meters = self.signed_gps_altitude();

        if let Some((width, height)) = self.best_dimensions()
            && dimensions_area(width, height) > dimensions_area(metadata.width, metadata.height)
        {
            metadata.width = width;
            metadata.height = height;
        }

        fill_missing(&mut metadata.camera_make, self.camera_make);
        fill_missing(&mut metadata.camera_model, self.camera_model);
        fill_missing(&mut metadata.lens_make, self.lens_make);
        fill_missing(&mut metadata.lens_model, self.lens_model);
        fill_missing(&mut metadata.f_number, self.f_number);
        fill_missing(
            &mut metadata.exposure_time_seconds,
            self.exposure_time_seconds,
        );
        fill_missing(&mut metadata.iso, self.iso);
        fill_missing(&mut metadata.exposure_bias_ev, self.exposure_bias_ev);
        fill_missing(&mut metadata.flash_mode, self.flash_mode);
        if metadata.flash_fired.is_none() {
            metadata.flash_fired = metadata.flash_mode.map(|mode| mode & 1 == 1);
        }
        fill_missing(&mut metadata.focal_length_mm, self.focal_length_mm);
        fill_missing(&mut metadata.focal_length_35mm, self.focal_length_35mm);
        fill_missing(&mut metadata.taken_at, taken_at);
        fill_missing(&mut metadata.orientation, self.orientation);
        fill_missing(&mut metadata.gps_latitude, gps_latitude);
        fill_missing(&mut metadata.gps_longitude, gps_longitude);
        fill_missing(&mut metadata.gps_altitude_meters, gps_altitude_meters);
        fill_missing(&mut metadata.artist, self.artist);
        fill_missing(&mut metadata.copyright, self.copyright);
        fill_missing(&mut metadata.software, self.software);
    }

    fn best_dimensions(&self) -> Option<(u32, u32)> {
        self.dimensions
            .iter()
            .copied()
            .filter(|(width, height)| *width > 0 && *height > 0)
            .max_by_key(|(width, height)| dimensions_area(*width, *height))
    }

    fn taken_at(&self) -> Option<String> {
        self.date_time_original
            .as_deref()
            .and_then(|value| {
                format_tiff_datetime(
                    value,
                    self.offset_time_original
                        .as_deref()
                        .or(self.offset_time.as_deref()),
                )
            })
            .or_else(|| {
                self.create_date.as_deref().and_then(|value| {
                    format_tiff_datetime(
                        value,
                        self.offset_time_digitized
                            .as_deref()
                            .or(self.offset_time.as_deref()),
                    )
                })
            })
            .or_else(|| {
                self.modify_date
                    .as_deref()
                    .and_then(|value| format_tiff_datetime(value, self.offset_time.as_deref()))
            })
    }

    fn signed_gps_latitude(&self) -> Option<f64> {
        signed_gps_coordinate(self.gps_latitude?, self.gps_latitude_ref.as_deref(), "S")
    }

    fn signed_gps_longitude(&self) -> Option<f64> {
        signed_gps_coordinate(self.gps_longitude?, self.gps_longitude_ref.as_deref(), "W")
    }

    fn signed_gps_altitude(&self) -> Option<f64> {
        let altitude = self.gps_altitude_meters?;
        Some(if self.gps_altitude_ref == Some(1) {
            -altitude
        } else {
            altitude
        })
    }
}

fn fill_missing<T>(target: &mut Option<T>, value: Option<T>) {
    if target.is_none() {
        *target = value;
    }
}

fn parse_tiff_fallback_metadata(bytes: &[u8]) -> Option<TiffFallbackMetadata> {
    if bytes.len() < 8 {
        return None;
    }

    let endian = match bytes.get(..2)? {
        b"II" => TiffEndian::Little,
        b"MM" => TiffEndian::Big,
        _ => return None,
    };

    let magic = endian.read_u16(bytes.get(2..4)?)?;
    let (bigtiff, first_ifd_offset) = match magic {
        42 => (false, u64::from(endian.read_u32(bytes.get(4..8)?)?)),
        43 => {
            if endian.read_u16(bytes.get(4..6)?)? != 8 || endian.read_u16(bytes.get(6..8)?)? != 0 {
                return None;
            }
            (true, endian.read_u64(bytes.get(8..16)?)?)
        }
        _ => return None,
    };

    let mut metadata = TiffFallbackMetadata::default();
    let mut queue = VecDeque::from([TiffIfdTask {
        offset: first_ifd_offset,
        kind: TiffIfdKind::Main,
        depth: 0,
    }]);
    let mut visited = HashSet::new();

    while let Some(task) = queue.pop_front() {
        if task.offset == 0 || task.depth > TIFF_FALLBACK_MAX_DEPTH || !visited.insert(task.offset)
        {
            continue;
        }
        parse_tiff_ifd(bytes, endian, bigtiff, task, &mut metadata, &mut queue);
    }

    Some(metadata)
}

fn parse_tiff_ifd(
    bytes: &[u8],
    endian: TiffEndian,
    bigtiff: bool,
    task: TiffIfdTask,
    metadata: &mut TiffFallbackMetadata,
    queue: &mut VecDeque<TiffIfdTask>,
) -> Option<()> {
    let offset = usize::try_from(task.offset).ok()?;
    let count_size = if bigtiff { 8 } else { 2 };
    let entry_size = if bigtiff { 20 } else { 12 };
    let offset_field_size = if bigtiff { 8 } else { 4 };
    let count_bytes = bytes.get(offset..offset.checked_add(count_size)?)?;
    let entry_count = if bigtiff {
        usize::try_from(endian.read_u64(count_bytes)?).ok()?
    } else {
        usize::from(endian.read_u16(count_bytes)?)
    };
    let entries_start = offset.checked_add(count_size)?;
    let entries_len = entry_count.checked_mul(entry_size)?;
    let entries_end = entries_start.checked_add(entries_len)?;
    if entries_end > bytes.len() {
        return None;
    }

    let mut width = None;
    let mut height = None;
    for index in 0..entry_count {
        let entry_offset = entries_start.checked_add(index.checked_mul(entry_size)?)?;
        let Some(entry) = parse_tiff_entry(
            bytes,
            endian,
            bigtiff,
            entry_offset,
            entry_size,
            offset_field_size,
        ) else {
            continue;
        };
        apply_tiff_entry(
            entry,
            task.kind,
            task.depth,
            metadata,
            queue,
            &mut width,
            &mut height,
        );
    }

    if let Some(dimensions) = width.zip(height) {
        metadata.dimensions.push(dimensions);
    }

    let next_offset_start = entries_end;
    let next_offset_end = next_offset_start.checked_add(offset_field_size)?;
    let next_offset_bytes = bytes.get(next_offset_start..next_offset_end)?;
    let next_offset = if bigtiff {
        endian.read_u64(next_offset_bytes)?
    } else {
        u64::from(endian.read_u32(next_offset_bytes)?)
    };
    enqueue_tiff_ifd(queue, next_offset, TiffIfdKind::Generic, task.depth + 1);
    Some(())
}

fn parse_tiff_entry(
    bytes: &[u8],
    endian: TiffEndian,
    bigtiff: bool,
    entry_offset: usize,
    entry_size: usize,
    offset_field_size: usize,
) -> Option<TiffEntry> {
    let entry = bytes.get(entry_offset..entry_offset.checked_add(entry_size)?)?;
    let tag = endian.read_u16(entry.get(0..2)?)?;
    let field_type = endian.read_u16(entry.get(2..4)?)?;
    let count = if bigtiff {
        endian.read_u64(entry.get(4..12)?)?
    } else {
        u64::from(endian.read_u32(entry.get(4..8)?)?)
    };
    let value_field_start = if bigtiff { 12 } else { 8 };
    let value_field = entry.get(value_field_start..value_field_start + offset_field_size)?;
    let value_data = tiff_entry_data(bytes, endian, field_type, count, value_field)?;
    let value = parse_tiff_entry_value(field_type, count, value_data, endian)?;
    Some(TiffEntry { tag, value })
}

fn tiff_entry_data<'a>(
    bytes: &'a [u8],
    endian: TiffEndian,
    field_type: u16,
    count: u64,
    value_field: &'a [u8],
) -> Option<&'a [u8]> {
    let byte_len = u64::from(tiff_field_type_byte_len(field_type)?);
    let total_len = byte_len.checked_mul(count)?;
    let total_len_usize = usize::try_from(total_len).ok()?;
    if total_len_usize <= value_field.len() {
        return value_field.get(..total_len_usize);
    }
    let data_offset = if value_field.len() == 8 {
        endian.read_u64(value_field)?
    } else {
        u64::from(endian.read_u32(value_field)?)
    };
    let data_offset = usize::try_from(data_offset).ok()?;
    bytes.get(data_offset..data_offset.checked_add(total_len_usize)?)
}

fn tiff_field_type_byte_len(field_type: u16) -> Option<u8> {
    match field_type {
        1 | 2 | 6 | 7 => Some(1),
        3 | 8 => Some(2),
        4 | 9 | 11 | 13 => Some(4),
        5 | 10 | 12 | 16 | 17 | 18 => Some(8),
        _ => None,
    }
}

fn parse_tiff_entry_value(
    field_type: u16,
    count: u64,
    data: &[u8],
    endian: TiffEndian,
) -> Option<TiffEntryValue> {
    let count = usize::try_from(count).ok()?;
    match field_type {
        1 | 7 => Some(TiffEntryValue::Bytes(data.to_vec())),
        2 => {
            let trimmed = data.split(|byte| *byte == 0).next().unwrap_or(data);
            let value = std::str::from_utf8(trimmed).ok()?.to_string();
            Some(TiffEntryValue::Ascii(value))
        }
        3 => Some(TiffEntryValue::U16(
            (0..count)
                .filter_map(|index| endian.read_u16(data.get(index * 2..index * 2 + 2)?))
                .collect(),
        )),
        4 | 13 => Some(TiffEntryValue::U32(
            (0..count)
                .filter_map(|index| endian.read_u32(data.get(index * 4..index * 4 + 4)?))
                .collect(),
        )),
        5 => Some(TiffEntryValue::URational(
            (0..count)
                .filter_map(|index| {
                    let start = index * 8;
                    Some((
                        endian.read_u32(data.get(start..start + 4)?)?,
                        endian.read_u32(data.get(start + 4..start + 8)?)?,
                    ))
                })
                .collect(),
        )),
        6 => Some(TiffEntryValue::I16(
            data.iter()
                .take(count)
                .map(|value| i16::from(i8::from_ne_bytes([*value])))
                .collect(),
        )),
        8 => Some(TiffEntryValue::I16(
            (0..count)
                .filter_map(|index| endian.read_i16(data.get(index * 2..index * 2 + 2)?))
                .collect(),
        )),
        9 => Some(TiffEntryValue::I32(
            (0..count)
                .filter_map(|index| endian.read_i32(data.get(index * 4..index * 4 + 4)?))
                .collect(),
        )),
        10 => Some(TiffEntryValue::IRational(
            (0..count)
                .filter_map(|index| {
                    let start = index * 8;
                    Some((
                        endian.read_i32(data.get(start..start + 4)?)?,
                        endian.read_i32(data.get(start + 4..start + 8)?)?,
                    ))
                })
                .collect(),
        )),
        11 => Some(TiffEntryValue::F32(
            (0..count)
                .filter_map(|index| endian.read_f32(data.get(index * 4..index * 4 + 4)?))
                .collect(),
        )),
        12 => Some(TiffEntryValue::F64(
            (0..count)
                .filter_map(|index| endian.read_f64(data.get(index * 8..index * 8 + 8)?))
                .collect(),
        )),
        16 | 18 => Some(TiffEntryValue::U64(
            (0..count)
                .filter_map(|index| endian.read_u64(data.get(index * 8..index * 8 + 8)?))
                .collect(),
        )),
        17 => Some(TiffEntryValue::I64(
            (0..count)
                .filter_map(|index| endian.read_i64(data.get(index * 8..index * 8 + 8)?))
                .collect(),
        )),
        _ => None,
    }
}

fn apply_tiff_entry(
    entry: TiffEntry,
    kind: TiffIfdKind,
    depth: usize,
    metadata: &mut TiffFallbackMetadata,
    queue: &mut VecDeque<TiffIfdTask>,
    width: &mut Option<u32>,
    height: &mut Option<u32>,
) {
    match entry.tag {
        TIFF_TAG_IMAGE_WIDTH | TIFF_TAG_EXIF_IMAGE_WIDTH => {
            if width.is_none() {
                *width = entry.value.first_u32();
            }
        }
        TIFF_TAG_IMAGE_LENGTH | TIFF_TAG_EXIF_IMAGE_HEIGHT => {
            if height.is_none() {
                *height = entry.value.first_u32();
            }
        }
        TIFF_TAG_MAKE => fill_missing(&mut metadata.camera_make, entry.value.string()),
        TIFF_TAG_MODEL => fill_missing(&mut metadata.camera_model, entry.value.string()),
        TIFF_TAG_ORIENTATION => fill_missing(&mut metadata.orientation, entry.value.first_u16()),
        TIFF_TAG_SOFTWARE => fill_missing(&mut metadata.software, entry.value.string()),
        TIFF_TAG_ARTIST => fill_missing(&mut metadata.artist, entry.value.string()),
        TIFF_TAG_COPYRIGHT => fill_missing(&mut metadata.copyright, entry.value.string()),
        TIFF_TAG_MODIFY_DATE => fill_missing(&mut metadata.modify_date, entry.value.string()),
        TIFF_TAG_EXIF_IFD => {
            enqueue_tiff_ifd_values(queue, &entry.value, TiffIfdKind::Exif, depth + 1)
        }
        TIFF_TAG_GPS_IFD => {
            enqueue_tiff_ifd_values(queue, &entry.value, TiffIfdKind::Gps, depth + 1)
        }
        TIFF_TAG_SUB_IFD => {
            enqueue_tiff_ifd_values(queue, &entry.value, TiffIfdKind::Generic, depth + 1)
        }
        TIFF_TAG_EXPOSURE_TIME => {
            fill_missing(&mut metadata.exposure_time_seconds, entry.value.first_f64())
        }
        TIFF_TAG_F_NUMBER => fill_missing(&mut metadata.f_number, entry.value.first_f64()),
        TIFF_TAG_ISO => fill_missing(&mut metadata.iso, entry.value.first_u32()),
        TIFF_TAG_DATE_TIME_ORIGINAL => {
            fill_missing(&mut metadata.date_time_original, entry.value.string());
        }
        TIFF_TAG_CREATE_DATE => fill_missing(&mut metadata.create_date, entry.value.string()),
        TIFF_TAG_OFFSET_TIME => fill_missing(&mut metadata.offset_time, entry.value.string()),
        TIFF_TAG_OFFSET_TIME_ORIGINAL => {
            fill_missing(&mut metadata.offset_time_original, entry.value.string());
        }
        TIFF_TAG_OFFSET_TIME_DIGITIZED => {
            fill_missing(&mut metadata.offset_time_digitized, entry.value.string());
        }
        TIFF_TAG_EXPOSURE_BIAS => {
            fill_missing(&mut metadata.exposure_bias_ev, entry.value.first_f64());
        }
        TIFF_TAG_FLASH => fill_missing(&mut metadata.flash_mode, entry.value.first_u16()),
        TIFF_TAG_FOCAL_LENGTH => {
            fill_missing(&mut metadata.focal_length_mm, entry.value.first_f64());
        }
        TIFF_TAG_FOCAL_LENGTH_35MM => {
            fill_missing(&mut metadata.focal_length_35mm, entry.value.first_u32());
        }
        TIFF_TAG_LENS_MAKE => fill_missing(&mut metadata.lens_make, entry.value.string()),
        TIFF_TAG_LENS_MODEL => fill_missing(&mut metadata.lens_model, entry.value.string()),
        TIFF_GPS_LATITUDE_REF if kind == TiffIfdKind::Gps => {
            fill_missing(&mut metadata.gps_latitude_ref, entry.value.string());
        }
        TIFF_GPS_LATITUDE if kind == TiffIfdKind::Gps => {
            fill_missing(
                &mut metadata.gps_latitude,
                entry.value.gps_decimal_degrees(),
            );
        }
        TIFF_GPS_LONGITUDE_REF if kind == TiffIfdKind::Gps => {
            fill_missing(&mut metadata.gps_longitude_ref, entry.value.string());
        }
        TIFF_GPS_LONGITUDE if kind == TiffIfdKind::Gps => {
            fill_missing(
                &mut metadata.gps_longitude,
                entry.value.gps_decimal_degrees(),
            );
        }
        TIFF_GPS_ALTITUDE_REF if kind == TiffIfdKind::Gps => {
            fill_missing(&mut metadata.gps_altitude_ref, entry.value.first_u8());
        }
        TIFF_GPS_ALTITUDE if kind == TiffIfdKind::Gps => {
            fill_missing(&mut metadata.gps_altitude_meters, entry.value.first_f64());
        }
        _ => {}
    }
}

fn enqueue_tiff_ifd_values(
    queue: &mut VecDeque<TiffIfdTask>,
    value: &TiffEntryValue,
    kind: TiffIfdKind,
    depth: usize,
) {
    for offset in value
        .offsets()
        .into_iter()
        .filter(|offset| *offset > 0)
        .take(TIFF_FALLBACK_MAX_OFFSETS_PER_TAG)
    {
        enqueue_tiff_ifd(queue, offset, kind, depth);
    }
}

fn enqueue_tiff_ifd(
    queue: &mut VecDeque<TiffIfdTask>,
    offset: u64,
    kind: TiffIfdKind,
    depth: usize,
) {
    if offset > 0 && depth <= TIFF_FALLBACK_MAX_DEPTH {
        queue.push_back(TiffIfdTask {
            offset,
            kind,
            depth,
        });
    }
}

fn rational_to_f64(numerator: u32, denominator: u32) -> Option<f64> {
    (denominator != 0).then_some(f64::from(numerator) / f64::from(denominator))
}

fn signed_rational_to_f64(numerator: i32, denominator: i32) -> Option<f64> {
    (denominator != 0).then_some(f64::from(numerator) / f64::from(denominator))
}

fn signed_gps_coordinate(value: f64, reference: Option<&str>, negative_ref: &str) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    Some(
        if reference
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case(negative_ref))
        {
            -value
        } else {
            value
        },
    )
}

fn format_tiff_datetime(value: &str, offset: Option<&str>) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(offset) = offset.and_then(|value| clean_metadata_string(Some(value))) {
        let with_offset = format!("{value} {offset}");
        if let Ok(datetime) =
            chrono::DateTime::parse_from_str(&with_offset, "%Y:%m:%d %H:%M:%S %:z")
                .or_else(|_| chrono::DateTime::parse_from_str(&with_offset, "%Y:%m:%d %H:%M:%S %z"))
        {
            return Some(datetime.to_rfc3339());
        }
    }

    chrono::NaiveDateTime::parse_from_str(value, "%Y:%m:%d %H:%M:%S")
        .ok()
        .map(|datetime| datetime.format("%Y-%m-%dT%H:%M:%S").to_string())
}
