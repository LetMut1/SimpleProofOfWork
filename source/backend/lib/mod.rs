#![allow(
    clippy::collapsible_else_if,
    clippy::collapsible_match,
    clippy::explicit_into_iter_loop,
    clippy::module_inception,
    clippy::needless_continue,
    clippy::needless_lifetimes,
    clippy::needless_return,
    clippy::new_without_default,
    clippy::redundant_pattern_matching,
    clippy::single_match_else,
    clippy::string_add,
    clippy::too_many_arguments,
    clippy::trait_duplication_in_bounds,
    clippy::unused_unit,
    clippy::empty_enum,
    clippy::let_unit_value
)]
#![deny(
    clippy::unnecessary_cast,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::fallible_impl_from,
    clippy::float_cmp_const,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]

pub use self::crypto::*;
pub use self::encode::*;
pub use self::error::*;
pub use self::protocol::*;
pub use self::word_of_wisdom::*;
pub use rand;
pub use serde;
pub use tokio;
pub use uuid;

pub const SERVER_SOCKET_ADDRESS: &'static str = "127.0.0.1:80";

mod protocol {
    use super::ErrorKind;
    use super::Secret;
    use serde::Deserialize;
    use serde::Serialize;
    use std::borrow::Cow;
    use std::error::Error;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use uuid::Uuid;

    pub struct Protocol;

    impl Protocol {
        const MAXIMUM_FRAME_SIZE: u8 = 2 ^ 6;
        const MAXIMUM_FRAMES_QUANTITY: u16 = u16::MAX;
        const MAXIMUM_BUFFER_SIZE: u64 =
            (Self::MAXIMUM_FRAMES_QUANTITY as u64) * (Self::MAXIMUM_FRAME_SIZE as u64);
        const QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION: u8 = 8;

        pub async fn send<'a>(
            tcp_stream: &'a mut TcpStream,
            mut data: Vec<u8>,
        ) -> Result<(), Box<dyn Error + 'static>> {
            let buffer_size = (Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION as u64)
                + (data.len() as u64);

            if buffer_size > (Self::MAXIMUM_BUFFER_SIZE) {
                return Err(Box::from(ErrorKind::RunTime));
            }

            let mut buffer = buffer_size.to_be_bytes().to_vec();

            buffer.append(&mut data);

            if let Err(error) = tcp_stream.write_all(buffer.as_slice()).await {
                return Err(Box::from(error));
            }

            return Ok(());
        }

        pub async fn receive<'a>(
            tcp_stream: &'a mut TcpStream,
        ) -> Result<Vec<u8>, Box<dyn Error + 'static>> {
            let mut buffer: Vec<u8> = vec![];

            let mut temporary_buffer: Vec<u8> = vec![];

            let mut buffer_size: Option<u64> = None;

            'a: loop {
                temporary_buffer.clear();

                match tcp_stream.read_buf(&mut temporary_buffer).await {
                    Ok(bytes_quantity) => {
                        if bytes_quantity == 0 {
                            break 'a;
                        }

                        match buffer_size {
                            Some(buffer_size_) => {
                                buffer.append(&mut temporary_buffer);

                                let current_buffer_size = buffer.len() as u64;

                                if current_buffer_size < buffer_size_ {
                                    continue 'a;
                                }

                                if current_buffer_size == buffer_size_ {
                                    break 'a;
                                }

                                if current_buffer_size > buffer_size_ {
                                    return Err(Box::from(ErrorKind::Logic));
                                }
                            }
                            None => {
                                if bytes_quantity
                                    < (Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION
                                        as usize)
                                {
                                    return Err(Box::from(ErrorKind::Logic));
                                }

                                let buffer_part: [u8; Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION as usize] =
                                    match temporary_buffer[..(Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION as usize)].try_into()
                                {
                                    Ok(buffer_part_) => buffer_part_,
                                    Err(error) => {
                                        return Err(Box::from(error));
                                    }
                                };

                                let mut buffer_size_ = u64::from_be_bytes(buffer_part);

                                if buffer_size_ > Self::MAXIMUM_BUFFER_SIZE {
                                    return Err(Box::from(ErrorKind::Logic));
                                }

                                let mut temporary_buffer_ = temporary_buffer[
                                    (Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION as usize)..
                                ].to_vec();

                                buffer.append(&mut temporary_buffer_);

                                buffer_size_ = match buffer_size_.checked_sub(
                                    Self::QUANTITY_OF_BYTES_FOR_BUFFER_SIZE_REPRESENTATION as u64,
                                ) {
                                    Some(buffer_size__) => buffer_size__,
                                    None => {
                                        return Err(Box::from(ErrorKind::Logic));
                                    }
                                };

                                buffer_size = Some(buffer_size_);

                                let current_buffer_size = buffer.len() as u64;

                                if current_buffer_size < buffer_size_ {
                                    continue 'a;
                                }

                                if current_buffer_size == buffer_size_ {
                                    break 'a;
                                }

                                if current_buffer_size > buffer_size_ {
                                    return Err(Box::from(ErrorKind::Logic));
                                }
                            }
                        }
                    }
                    Err(error) => {
                        return Err(Box::from(error));
                    }
                }
            }

            return Ok(buffer);
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Request<'a> {
        Challenge { token: Cow<'a, Uuid> },
        WordOfWisdom { token: Cow<'a, Uuid>, result: u64 },
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Response<'a> {
        Challenge { secret: Secret },
        WordOfWisdom { word_of_wisdom: WordOfWisdom<'a> },
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum WordOfWisdom<'a> {
        Result { result: Cow<'a, str> },
        Fail,
    }
}

mod encode {
    use rmp_serde::encode::write;
    use rmp_serde::from_read_ref;
    use rmp_serde::to_vec;
    use serde::Deserialize;
    use serde::Serialize;
    use std::error::Error;

    pub struct Serializer;

    impl Serializer {
        pub fn serialize<'a, T>(subject: &'a T) -> Result<Vec<u8>, Box<dyn Error + 'static>>
        where
            T: Serialize,
        {
            let data = match to_vec(subject) {
                Ok(data_) => data_,
                Err(error) => {
                    return Err(Box::from(error));
                }
            };

            return Ok(data);
        }

        pub fn serialize_<'a, T>(
            subject: &'a T,
            buffer: &'a mut Vec<u8>,
        ) -> Result<(), Box<dyn Error + 'static>>
        where
            T: Serialize,
        {
            if let Err(error) = write(buffer, subject) {
                return Err(Box::from(error));
            }

            return Ok(());
        }

        pub fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, Box<dyn Error + 'static>>
        where
            T: Deserialize<'a>,
        {
            let subject = match from_read_ref::<'_, [u8], T>(data) {
                Ok(subject_) => subject_,
                Err(error) => {
                    return Err(Box::from(error));
                }
            };

            return Ok(subject);
        }
    }
}

mod crypto {
    use super::Serializer;
    use crypto::digest::Digest;
    use crypto::sha2::Sha256;
    use rand::thread_rng;
    use rand::Rng;
    use serde::Deserialize;
    use serde::Serialize;
    use std::error::Error;
    use uuid::Uuid;

    pub struct POW {
        sha256: Sha256,
        difficulty: Difficulty,
        result_hash: Vec<u8>,
    }

    impl POW {
        pub const DEFAULT_DIFFICULTY: Difficulty = Difficulty::III;

        pub fn new(difficulty: Difficulty) -> Self {
            let sha256 = Sha256::new();

            let bytes_quantity = sha256.output_bytes();

            return Self {
                sha256,
                difficulty,
                result_hash: vec![0; bytes_quantity],
            };
        }

        pub fn find_nonce<'a>(
            &'a mut self,
            secret: &'a Secret,
        ) -> Result<u64, Box<dyn Error + 'static>> {
            let data = Serializer::serialize(secret)?;

            let random_number = 'a: loop {
                let random_number_ = thread_rng().gen_range(0..u64::MAX);

                if self.verify_nonce_(data.as_slice(), random_number_) {
                    break 'a random_number_;
                }

                continue 'a;
            };

            return Ok(random_number);
        }

        pub fn verify_nonce<'a>(
            &'a mut self,
            secret: &'a Secret,
            nonce: u64,
        ) -> Result<bool, Box<dyn Error + 'static>> {
            let data = Serializer::serialize(secret)?;

            return Ok(self.verify_nonce_(data.as_slice(), nonce));
        }

        fn verify_nonce_<'a>(&'a mut self, secret: &'a [u8], nonce: u64) -> bool {
            let mut buffer: Vec<u8> = vec![];

            buffer.extend_from_slice(secret);

            buffer.extend_from_slice(nonce.to_be_bytes().as_slice());

            let zero_bytes_quantity = self.difficulty.into_number();

            self.sha256.reset();

            self.sha256.input(buffer.as_slice());

            self.sha256.result(self.result_hash.as_mut_slice());

            self.sha256.reset();

            self.sha256.input(self.result_hash.as_slice());

            self.sha256.result(self.result_hash.as_mut_slice());

            let mut zero_bytes_quantity_counter: usize = 0;

            for i in 0..zero_bytes_quantity {
                if self.result_hash[i] == 0 {
                    zero_bytes_quantity_counter += 1;
                }
            }

            if zero_bytes_quantity_counter == zero_bytes_quantity {
                return true;
            }

            return false;
        }
    }

    pub enum Difficulty {
        I,
        // < 10 sec.
        II,
        // < 5 min.
        III,
        IV,
    }

    impl Difficulty {
        fn into_number<'a>(&'a self) -> usize {
            return match *self {
                Self::I => 1,
                Self::II => 2,
                Self::III => 3,
                Self::IV => 4,
            };
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Secret {
        pub value: Uuid,
    }
}

mod error {
    use std::error::Error;
    use std::fmt::Display;
    use std::fmt::Formatter;
    use std::fmt::Result as FmrResult;

    #[derive(Debug)]
    pub enum ErrorKind {
        RunTime,
        Logic,
    }

    impl Display for ErrorKind {
        fn fmt<'a>(&'a self, formatter: &'a mut Formatter<'_>) -> FmrResult {
            match *self {
                Self::Logic => {
                    writeln!(formatter, "Logic error.")
                }
                Self::RunTime => {
                    writeln!(formatter, "RunTime error.")
                }
            }
        }
    }

    impl Error for ErrorKind {}
}

mod word_of_wisdom {
    pub const WORD_OF_WISDOM_QUOTES: [&'static str; 10] = [
        "You create your own opportunities. Success doesn`t just come and find you-you have to go out and get it.",
        "Never break your promises. Keep every promise; it makes you credible.",
        "You are never as stuck as you think you are. Success is not final, and failure isn`t fatal.",
        "Happiness is a choice. For every minute you are angry, you lose 60 seconds of your own happiness.",
        "Habits develop into character. Character is the result of our mental attitude and the way we spend our time.",
        "Be happy with who you are. Being happy doesn`t mean everything is perfect but that you have decided to look beyond the imperfections.",
        "Don`t seek happiness-create it. You don`t need life to go your way to be happy.",
        "If you want to be happy, stop complaining. If you want happiness, stop complaining about how your life isn`t what you want and make it into what you do want.",
        "Asking for help is a sign of strength. Don`t let your fear of being judged stop you from asking for help when you need it. Sometimes asking for help is the bravest move you can make. You don`t have to go it alone.",
        "Replace every negative thought with a positive one. A positive mind is stronger than a negative thought."
    ];
}
