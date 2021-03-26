use reqwest::{Client, Result, Method};
use roxmltree::{Document, Children, Descendants, Node};
use crate::util::util;

// pub async fn get_posts<'input>(tags: Option<Vec<&str>>) -> Document<'input> {
//     let client = Client::new();
//
//     return if tags.is_some() {
//         let tag_list = tags.unwrap().join(",");
//         let resp = client.request(Method::GET, "https://safebooru.org/index.php?page=dapi&s=post&q=index")
//             .query(&[("tags", tag_list)])
//             .send()
//             .await
//             .unwrap()
//             .text()
//             .await
//             .unwrap();
//
//         let doc = Document::parse(resp.as_str()).unwrap();
//         doc
//     } else {
//         let resp = client.request(Method::GET, "https://safebooru.org/index.php?page=dapi&s=post&q=index")
//             .send()
//             .await
//             .unwrap()
//             .text()
//             .await
//             .unwrap();
//
//         Document::parse(resp.as_str()).unwrap()
//     }
// }


async fn get_random_post(tags: Option<Vec<&str>>) -> String {
    let client = Client::new();

    let resp = if tags.is_some() {
        let tag_list = tags.unwrap().join(",");
        client.request(Method::GET, "https://safebooru.org/index.php?page=dapi&s=post&q=index")
            .query(&[("tags", tag_list)])
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    } else {
        client.request(Method::GET, "https://safebooru.org/index.php?page=dapi&s=post&q=index")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    };
    let posts = Document::parse(resp.as_str()).unwrap();
    let num_posts = posts.root().descendants().
        filter(|node| node.tag_name().name().eq("post"))
        .count();

    posts.root().descendants().filter(|node| node.tag_name().name().eq("post"))
        .nth(util::get_rand_num(0, num_posts))
        .unwrap()
        .attribute("file_url")
        .unwrap()
        .to_string()
}