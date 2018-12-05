#![feature(try_from)]

#[macro_use]
extern crate hdk;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate boolinator;
extern crate serde_json;
#[macro_use]
extern crate holochain_core_types_derive;

pub mod blog;
pub mod post;

use hdk::holochain_core_types::cas::content::Address;

define_zome! {
    entries: [
        post::definition()
    ]

    genesis: || {
        Ok(())
    }

    functions: {
        main (Public) {
            check_sum: {
                inputs: |num1: u32, num2: u32|,
                outputs: |post: JsonString|,
                handler: blog::handle_check_sum
            }

            hash_post: {
                inputs: |content: String|,
                outputs: |result: JsonString|,
                handler: blog::handle_hash_post
            }

            create_post: {
                inputs: |content: String, in_reply_to: Address|,
                outputs: |result: JsonString|,
                handler: blog::handle_create_post
            }

            posts_by_agent: {
                inputs: |agent: Address|,
                outputs: |post_hashes: Vec<Address>|,
                handler: blog::handle_posts_by_agent
            }

            get_post: {
                inputs: |post_address: Address|,
                outputs: |post: serde_json::Value|,
                handler: blog::handle_get_post
            }

            my_posts: {
                inputs: | |,
                outputs: |post_hashes: Vec<Address>|,
                handler: blog::handle_my_posts
            }

            my_posts_as_committed: {
                inputs: | |,
                outputs: |post_hashes: Vec<Address>|,
                handler: blog::handle_my_posts_as_commited
            }
        }
    }
}
