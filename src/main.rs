use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::str::{from_utf8};
use std::time::SystemTime;

use serde_json::{Value, json};
use curl::easy::{Easy, List};

static SAVE_FILE: &str = "bulk_dump.json";
static USER_AGENT_STR: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.192 Safari/537.36";

fn main () {
    let mut data_crawled_dump = json!({
        "pages": [],
        "stats": {
            "start_time": match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n.as_secs(),
                Err(_) => panic!("data_crawled_dump['stats']['start_time']: SystemTime before UNIX EPOCH!"),
            },
            "end_time": 0,
            "total_entries": 0
        },
        "failed_pages": []
    });

    let mut failed_pages: Vec<Value> = Vec::new();
    let mut total_entries: u64 = 0;

    let mut easy = Easy::new();
    
    let auth_token: String = match easy.url("https://www.yemeksepeti.com/ankara/kebap-turk-mutfagi") {
        Ok(_) => {
            easy.cookie_list("").unwrap();

            easy.write_function(|data| {
                Ok(data.len())
            }).unwrap();

            easy.perform().unwrap();
            
            match easy.cookies() {
                Ok(list) => {
                    match list.into_iter().collect::<Vec<&[u8]>>().into_iter().map(|x| match from_utf8(x) {
                        Ok(s) => s,
                        Err(e) => panic!("Error from from_utf8(): {}", e)
                    }).collect::<Vec<&str>>().into_iter().filter(|x| x.contains("oauth_anonym_token")).collect::<Vec<&str>>().last() {
                        Some(v) => String::from(match v.to_owned().split("\t").into_iter().collect::<Vec<&str>>().last() {
                            Some(x) => x.to_owned(),
                            None => panic!("No oauth_anonym_token returned from the cookie text")
                        }),
                        None => panic!("No oauth_anonym_token returned from the request")
                    }
                },
                Err(e) => panic!("Error from easy.url(): {}", e)
            }
        },
        Err(e) => panic!("Error from easy.url(): {}", e)
    };

    println!("+ Auth token: {}", auth_token);

    let mut page_number: u32 = 1;
    let mut data_crawled: Vec<Value> = Vec::new();

    loop {
        let mut headers = List::new();

        headers.append("authority: gate.yemeksepeti.com").unwrap();
        headers.append("accept: */*").unwrap();
        headers.append("ys-catalog: TR_ANKARA").unwrap();
        headers.append(format!("authorization: Bearer {}", auth_token).as_str()).unwrap();
        headers.append("ys-culture: tr-TR").unwrap();
        headers.append("content-type: application/json;charset=UTF-8").unwrap();
        headers.append("origin: https://www.yemeksepeti.com").unwrap();
        headers.append("sec-fetch-site: same-site").unwrap();
        headers.append("sec-fetch-mode: cors").unwrap();
        headers.append("sec-fetch-dest: empty").unwrap();
        headers.append("referer: https://www.yemeksepeti.com/").unwrap();
        headers.append("accept-language: en-US,en;q=0.9").unwrap();
    
        easy.http_headers(headers).unwrap();
        easy.useragent(USER_AGENT_STR).unwrap();
    
        match easy.url(format!("https://gate.yemeksepeti.com/discovery/api/v1/Restaurant/search?PageNumber={}&SortField=1&SortDirection=0&OpenOnly=false&AreaId=&CuisineId=53bf4e74-469a-48af-b6ea-8523a2c3a018&PageSize=50", page_number).as_str()) {
            Err(e) => panic!("Error from easy.url(): {}", e),
            _ => ()
        };
    
        let mut buf = String::new();
        let mut all_okay: bool = true;

        {
            let mut transfer = easy.transfer();
            transfer.write_function(|data| {
                match from_utf8(data.clone()) {
                    Ok(v) => buf.extend(String::from(v).chars()),
                    Err(e) => {
                        println!("[!] Page {} error: from_utf8(): {}", page_number, e);
                        all_okay = false
                    }
                };

                Ok(data.len())
            }).unwrap();
    
            transfer.perform().unwrap();
        }
    
        if all_okay {
            let page_data: Value = serde_json::from_str(buf.as_str()).unwrap();

            match page_data {
                Value::Object(mut obj) => {
                    let page_size = obj.get("Data").unwrap().get("Result").unwrap().as_array().unwrap().len();
                    total_entries += page_size as u64;
    
                    println!("+ Page {} size: {}", page_number, page_size);
    
                    if page_size == 0 {
                        break;
                    } else {
                        obj.insert("$page".to_string(), json!(page_number));
                        data_crawled.push(Value::Object(obj));
                    }
                }
                _ => panic!("Response is not an object.")
            }
        } else {
            failed_pages.push(json!(page_number));
        }

        page_number += 1;
    }

    data_crawled_dump["pages"] = Value::Array(data_crawled);
    data_crawled_dump["failed_pages"] = Value::Array(failed_pages);
    data_crawled_dump["total_entries"] = json!(total_entries);

    let path = Path::new(SAVE_FILE);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("File::create(&path): couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    data_crawled_dump["stats"]["end_time"] = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => json!(n.as_secs()),
        Err(_) => panic!("data_crawled_dump['stats']['end_time']: SystemTime before UNIX EPOCH!"),
    };

    match file.write_all(data_crawled_dump.to_string().as_bytes()) {
        Err(why) => panic!("file.write_all(): couldn't write to {}: {}", display, why),
        Ok(_) => println!("\n[+] Successfully wrote to {}", display),
    }
}