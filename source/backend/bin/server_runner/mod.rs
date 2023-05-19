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

use infrastructure::rand::thread_rng;
use infrastructure::rand::Rng;
use infrastructure::tokio::net::TcpListener;
use infrastructure::tokio::net::TcpStream;
use infrastructure::tokio::runtime::Builder;
use infrastructure::tokio::spawn;
use infrastructure::uuid::Uuid;
use infrastructure::Protocol;
use infrastructure::Request;
use infrastructure::Response;
use infrastructure::Secret;
use infrastructure::Serializer;
use infrastructure::WordOfWisdom;
use infrastructure::POW;
use infrastructure::SERVER_SOCKET_ADDRESS;
use infrastructure::WORD_OF_WISDOM_QUOTES;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

fn main() -> () {
    if let Err(error) = process() {
        println!("{}", &error);
    }

    return ();
}

fn process() -> Result<(), Box<dyn Error + 'static>> {
    let runtime = match Builder::new_multi_thread().enable_all().build() {
        Ok(runtime_) => runtime_,
        Err(error) => {
            return Err(Box::from(error));
        }
    };

    if let Err(error) = runtime.block_on(run_tcp_server()) {
        return Err(Box::from(error));
    }

    return Ok(());
}

type RequestsState = Arc<Mutex<HashMap<Uuid, Secret>>>;

async fn run_tcp_server() -> Result<(), Box<dyn Error + 'static>> {
    let requests_state: RequestsState = Arc::new(Mutex::new(HashMap::new()));

    let tcp_listener = match TcpListener::bind(SERVER_SOCKET_ADDRESS).await {
        Ok(tcp_listener_) => tcp_listener_,
        Err(error) => {
            return Err(Box::from(error));
        }
    };

    loop {
        let tcp_stream = match tcp_listener.accept().await {
            Ok((tcp_stream_, _)) => tcp_stream_,
            Err(error) => {
                return Err(Box::from(error));
            }
        };

        spawn(handle_stream(tcp_stream, requests_state.clone()));
    }

    return Ok(());
}

async fn handle_stream(mut tcp_stream: TcpStream, requests_state: RequestsState) -> () {
    let data = match Protocol::receive(&mut tcp_stream).await {
        Ok(data_) => data_,
        Err(error) => {
            println!("{}", &error);

            return ();
        }
    };

    let request = match Serializer::deserialize::<'_, Request>(data.as_slice()) {
        Ok(request_) => request_,
        Err(error) => {
            println!("{}", &error);

            return ();
        }
    };

    match request {
        Request::Challenge { token } => {
            let secret = Secret {
                value: Uuid::new_v4(),
            };

            {
                let mut mutex_guard = match requests_state.lock() {
                    Ok(mutex_guard_) => mutex_guard_,
                    Err(error) => {
                        println!("{}", &error);

                        return ();
                    }
                };

                mutex_guard.insert(token.into_owned(), secret.clone());
            }

            let response = Response::Challenge { secret };

            let data = match Serializer::serialize(&response) {
                Ok(data_) => data_,
                Err(error) => {
                    println!("{}", &error);

                    return ();
                }
            };

            if let Err(error) = Protocol::send(&mut tcp_stream, data).await {
                println!("{}", &error);

                return ();
            }
        }
        Request::WordOfWisdom { token, result } => {
            let mut p_o_w = POW::new(POW::DEFAULT_DIFFICULTY);

            let all_right = {
                let mut mutex_guard = match requests_state.lock() {
                    Ok(mutex_guard_) => mutex_guard_,
                    Err(error) => {
                        println!("{}", &error);

                        return ();
                    }
                };

                match mutex_guard.get(token.as_ref()) {
                    Some(secret) => {
                        let result = match p_o_w.verify_nonce(secret, result) {
                            Ok(result_) => result_,
                            Err(error) => {
                                println!("{}", &error);

                                return ();
                            }
                        };

                        if result {
                            mutex_guard.remove(token.as_ref());
                        }

                        result
                    }
                    None => false,
                }
            };

            let word_of_wisdom = if all_right {
                let word_of_wisdom = WORD_OF_WISDOM_QUOTES[
                    thread_rng().gen_range::<usize, _>(0..WORD_OF_WISDOM_QUOTES.len())
                ];

                WordOfWisdom::Result {
                    result: Cow::Borrowed(word_of_wisdom),
                }
            } else {
                WordOfWisdom::Fail
            };

            let response = Response::WordOfWisdom { word_of_wisdom };

            let data = match Serializer::serialize(&response) {
                Ok(data_) => data_,
                Err(error) => {
                    println!("{}", &error);

                    return ();
                }
            };

            if let Err(error) = Protocol::send(&mut tcp_stream, data).await {
                println!("{}", &error);

                return ();
            }
        }
    }

    return ();
}
