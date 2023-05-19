#![allow(
    unreachable_code,
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

use infrastructure::tokio::net::TcpStream;
use infrastructure::tokio::runtime::Builder;
use infrastructure::uuid::Uuid;
use infrastructure::ErrorKind;
use infrastructure::Protocol;
use infrastructure::Request;
use infrastructure::Response;
use infrastructure::Secret;
use infrastructure::Serializer;
use infrastructure::WordOfWisdom;
use infrastructure::POW;
use infrastructure::SERVER_SOCKET_ADDRESS;
use std::borrow::Cow;
use std::convert::From;
use std::error::Error;

fn main() -> () {
    if let Err(error) = process() {
        println!("{}", &error);
    }

    return ();
}

fn process() -> Result<(), Box<dyn Error + 'static>> {
    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime_) => runtime_,
        Err(error) => {
            return Err(Box::from(error));
        }
    };

    if let Err(error) = runtime.block_on(communicate()) {
        return Err(Box::from(error));
    }

    return Ok(());
}

async fn communicate() -> Result<(), Box<dyn Error + 'static>> {
    let token = Uuid::new_v4();

    let secret = get_secret(&token).await?;

    let mut p_o_w = POW::new(POW::DEFAULT_DIFFICULTY);

    let nonce = p_o_w.find_nonce(&secret)?;

    let word_of_wisdom = get_word_of_wisdom(&token, nonce).await?;

    match word_of_wisdom {
        WordOfWisdom::Result { result } => {
            println!("{}", result);
        }
        WordOfWisdom::Fail => {
            println!("Failed. Work proof has been corrupted.");
        }
    }

    return Ok(());
}

async fn get_secret<'a>(token: &'a Uuid) -> Result<Secret, Box<dyn Error + 'static>> {
    let request = Request::Challenge {
        token: Cow::Borrowed(token),
    };

    let data = Serializer::serialize(&request)?;

    let mut tcp_stream = match TcpStream::connect(SERVER_SOCKET_ADDRESS).await {
        Ok(tcp_stream_) => tcp_stream_,
        Err(error) => {
            return Err(Box::from(error));
        }
    };

    Protocol::send(&mut tcp_stream, data).await?;

    let data = Protocol::receive(&mut tcp_stream).await?;

    let response = Serializer::deserialize::<'_, Response>(data.as_slice())?;

    let secret_ = match response {
        Response::Challenge { secret } => secret,
        Response::WordOfWisdom { word_of_wisdom: _ } => {
            return Err(Box::from(ErrorKind::Logic));
        }
    };

    return Ok(secret_);
}

async fn get_word_of_wisdom<'a>(
    token: &'a Uuid,
    nonce: u64,
) -> Result<WordOfWisdom, Box<dyn Error + 'static>> {
    let request = Request::WordOfWisdom {
        token: Cow::Borrowed(token),
        result: nonce,
    };

    let data = Serializer::serialize(&request)?;

    let mut tcp_stream = match TcpStream::connect(SERVER_SOCKET_ADDRESS).await {
        Ok(tcp_stream_) => tcp_stream_,
        Err(error) => {
            return Err(Box::from(error));
        }
    };

    Protocol::send(&mut tcp_stream, data).await?;

    let data = Protocol::receive(&mut tcp_stream).await?;

    let response = Serializer::deserialize::<'_, Response>(data.as_slice())?;

    let word_of_wisdom_ = match response {
        Response::Challenge { secret: _ } => {
            return Err(Box::from(ErrorKind::Logic));
        }
        Response::WordOfWisdom { word_of_wisdom } => word_of_wisdom,
    };

    return Ok(word_of_wisdom_);
}
