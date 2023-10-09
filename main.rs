use chrono;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Instant;
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug)]
struct OrderBook {
    bidinfo: BTreeMap<OrderedFloat<f64>, f64>,
    askinfo: BTreeMap<OrderedFloat<f64>, f64>,
    latestchangeid: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Priceinfo {
    status: String,
    price: f64,
    qty: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    r#type: String,
    timestamp: u64,
    instrument_name: String,
    change_id: u64,
    bids: Vec<Priceinfo>,
    asks: Vec<Priceinfo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Params {
    channel: String,
    data: Data,
}

#[derive(Deserialize, Debug)]
struct SnapshotMsg {
    method: String,
    params: Params,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChangeParams {
    channel: String,
    data: ChangeData,
}

#[derive(Deserialize, Debug)]
struct ChangeMsg {
    params: ChangeParams,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChangeData {
    r#type: String,
    timestamp: u64,
    prev_change_id: u64,
    instrument_name: String,
    change_id: u64,
    bids: Vec<Priceinfo>,
    asks: Vec<Priceinfo>,
}

fn updatebids(orderbook: &mut OrderBook, input: &Vec<Priceinfo>) {
    for item in input.iter() {
        if item.status == "new" {
            orderbook.bidinfo.insert(OrderedFloat(item.price), item.qty);
        } else if item.status == "delete" {
            orderbook.bidinfo.remove(&OrderedFloat(item.price));
        } else if item.status == "change" {
            orderbook.bidinfo.insert(OrderedFloat(item.price), item.qty);
        }
    }
}

fn updateasks(orderbook: &mut OrderBook, input: &Vec<Priceinfo>) {
    for item in input.iter() {
        if item.status == "new" {
            orderbook.askinfo.insert(OrderedFloat(item.price), item.qty);
        } else if item.status == "delete" {
            orderbook.askinfo.remove(&OrderedFloat(item.price));
        } else if item.status == "change" {
            orderbook.askinfo.insert(OrderedFloat(item.price), item.qty);
        }
    }
}
fn fetchorderbook(orderbook: &mut OrderBook) {
    let mut start = Instant::now();

    let (mut socket, _response) =
        connect(Url::parse("wss://test.deribit.com/den/ws").unwrap()).expect("Can't connect");

    socket
        .write_message(
            Message::Text(
                json!( {"jsonrpc": "2.0",
                 "method": "public/subscribe",
                 "id": 42,
                 "params": {
                    "channels": ["book.ETH-PERPETUAL.100ms"]}
                })
                .to_string(),
            )
            .into(),
        )
        .expect("Error sending message");

    let _response = socket.read_message().expect("Error reading message");
    let msg = socket.read_message().expect("Error reading message");
    let result: Result<SnapshotMsg, serde_json::Error> =
        serde_json::from_str(msg.to_text().unwrap());
    let _value = match result {
        Ok(ref msg) => {
            if msg.method == "subscription" {
                if msg.params.data.r#type == "snapshot" {
                    updatebids(orderbook, &msg.params.data.bids);
                    updateasks(orderbook, &msg.params.data.asks);
                    orderbook.latestchangeid = msg.params.data.change_id;
                }
            }
        }
        Err(_) => (),
    };

    loop {
        let msg = socket.read_message().expect("Error reading message");
        let result: Result<ChangeMsg, serde_json::Error> =
            serde_json::from_str(msg.to_text().unwrap());
        let _value = match result {
            Ok(ref msg) => {
                if orderbook.latestchangeid == msg.params.data.prev_change_id {
                    updatebids(orderbook, &msg.params.data.bids);
                    updateasks(orderbook, &msg.params.data.asks);
                    orderbook.latestchangeid = msg.params.data.change_id;
                } else {
                    fetchorderbook(orderbook);
                    return;
                }
            }
            Err(_) => (),
        };
        if start.elapsed().as_millis() > 500 {
            clearscreen::clear().expect("Error clearing screen");
            start = Instant::now();
            println!();
            println!("Symbol  : ETH-PERPETUAL");
            println!(
                "Time    : {}",
                chrono::offset::Local::now().format("%a %b %e %T %Y")
            );
            println!();
            println!("\tPrice \t\tQuantity");

            if let Some((key, _val)) = orderbook.bidinfo.last_key_value() {
                println!("BestBid\t{}\t\t{}", key, _val);
            }
            if let Some((key, _val)) = orderbook.askinfo.first_key_value() {
                println!("BestAsk\t{}\t\t{}", key, _val);
            }
        }
    }
}
fn main() {
    let bidinfo = BTreeMap::new();
    let askinfo = BTreeMap::new();
    let latestchangeid = 0;
    let mut orderbook = OrderBook {
        bidinfo,
        askinfo,
        latestchangeid,
    };
    fetchorderbook(&mut orderbook);
}
