use actix::*;
use failure::Error;
use futures::{future, Future, Stream};
use std::time::Duration;
use reqwest::r#async as async_http;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GetUrl(String);

impl Message for GetUrl {
    type Result = Result<String, Error>;
}

#[derive(Debug)]
struct HttpClient;

impl Actor for HttpClient {
    type Context = Context<Self>;
}

impl Handler<GetUrl> for HttpClient {
    type Result = ResponseFuture<String, Error>;

    fn handle(&mut self, msg: GetUrl, ctx: &mut Context<Self>) -> <Self as Handler<GetUrl>>::Result {
        println!("handle geturl");
        let client = async_http::Client::new();
        println!("create client");
        let future = client.get(&msg.0).send().map_err(Error::from).and_then(|res| {
            println!("{:?}", res);
            res.into_body().map_err(From::from).fold(String::new(), |mut buf, c| -> future::FutureResult<String, Error> {
                buf.push_str(&String::from_utf8_lossy(&c));
                future::ok(buf)
            })
        });
        println!("create future");
        Box::new(future.map_err(From::from))
    }
}

#[derive(Debug)]
struct App {
    http: Addr<HttpClient>
}

impl App {
    pub fn new(http: Addr<HttpClient>) -> App {
        App { http }
    }
}

impl Actor for App {
    type Context = Context<Self>;
}

struct AppStart;

impl Message for AppStart {
    type Result = Result<(), Error>;
}

impl Handler<AppStart> for App {
    type Result = ResponseFuture<(), Error>;

    /// メインの処理。
    /// 基本は1スレッドを占有し続ける事になりそう。
    /// SyncArbiterとして実装してもいいかも。
    fn handle(&mut self, _: AppStart, ctx: &mut Context<Self>) -> <Self as Handler<AppStart>>::Result {
        let future = self.http.send(GetUrl("https://www.pushcode.jp/".to_string()))//.timeout(Duration::from_secs(180))
            .and_then(|result| {
                match result {
                    Ok(html) => println!("{}", html),
                    Err(err) => eprintln!("{}", err),
                }
                println!("Done");
                System::current().stop();
                Ok(())
            })
            .or_else(|err| {
                eprintln!("{}", err);
                future::err(err)
            });
        Box::new(future.map_err(From::from))
    }
}

fn main() {
    let actor_system = System::new(env!("CARGO_PKG_NAME"));
    let http = HttpClient.start();
    let app = App::new(http.clone()).start();
    app.do_send(AppStart);
    actor_system.run();
}
