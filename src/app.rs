use futures::{Sink, Stream, StreamExt};
use http::Method;
use leptos::{html::Input, prelude::*, task::spawn_local};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use server_fn::{
    client::{browser::BrowserClient, Client},
    codec::{
        Encoding, FromReq, FromRes, GetUrl, IntoReq, IntoRes, MultipartData,
        MultipartFormData, Postcard, Rkyv, RkyvEncoding, SerdeLite,
        StreamingText, TextStream,
    },
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::{browser::BrowserRequest, ClientReq, Req},
    response::{browser::BrowserResponse, ClientRes, TryRes},
    ContentType, Format, FormatType,
};
use std::future::Future;
#[cfg(feature = "ssr")]
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use strum::{Display, EnumString};
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlFormElement, SubmitEvent};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <meta name="color-scheme" content="dark light" />
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico" />
                <link rel="stylesheet" id="leptos" href="/pkg/server_fns_axum.css" />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <header>
            <h1>"Server Function Demo"</h1>
        </header>
        <main>
            <HomePage />
        </main>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <h2>"Some Simple Server Functions"</h2>
        <SpawnLocal />
        <WithAnAction />
        <WithActionForm />
        <h2>"Custom Error Types"</h2>
        <CustomErrorTypes />
        <h2>"Alternative Encodings"</h2>
        <ServerFnArgumentExample />
        <RkyvExample />
        <PostcardExample />
        <FileUpload />
        <FileUploadWithProgress />
        <FileWatcher />
        <CustomEncoding />
        <CustomClientExample />
    }
}

#[component]
pub fn SpawnLocal() -> impl IntoView {
    #[server]
    pub async fn shouting_text(input: String) -> Result<String, ServerFnError> {
        // insert a simulated wait
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        Ok(input.to_ascii_uppercase())
    }

    let input_ref = NodeRef::<Input>::new();
    let (shout_result, set_shout_result) = signal("Click me".to_string());

    view! {
        <h3>Using <code>spawn_local</code></h3>
        <p>
            "You can call a server function by using " <code>"spawn_local"</code>
            " in an event listener. "
            "Clicking this button should alert with the uppercase version of the input."
        </p>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let uppercase_text = shouting_text(value).await.unwrap_or_else(|e| e.to_string());
                set_shout_result.set(uppercase_text);
            });
        }>

            {shout_result}
        </button>
    }
}

#[cfg(feature = "ssr")]
static ROWS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[server]
pub async fn add_row(text: String) -> Result<usize, ServerFnError> {
    static N: AtomicU8 = AtomicU8::new(0);

    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let nth_run = N.fetch_add(1, Ordering::Relaxed);
    // this will print on the server, like any server function
    println!("Adding {text:?} to the database!");
    if nth_run % 3 == 2 {
        Err(ServerFnError::new("Oh no! Couldn't add to database!"))
    } else {
        let mut rows = ROWS.lock().unwrap();
        rows.push(text);
        Ok(rows.len())
    }
}

#[server]
pub async fn get_rows() -> Result<usize, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    Ok(ROWS.lock().unwrap().len())
}

#[component]
pub fn WithAnAction() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();

    let action = ServerAction::<AddRow>::new();

    let row_count =
        Resource::new(move || action.version().get(), |_| get_rows());

    view! {
        <h3>Using <code>Action::new</code></h3>
        <p>
            "Some server functions are conceptually \"mutations,\", which change something on the server. "
            "These often work well as actions."
        </p>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let text = input_ref.get().unwrap().value();
            action.dispatch(text.into());
        }>

            Submit
        </button>
        <p>You submitted: {move || format!("{:?}", action.input().get())}</p>
        <p>The result was: {move || format!("{:?}", action.value().get())}</p>
        <Transition>
            <p>Total rows: {row_count}</p>
        </Transition>
    }
}

#[component]
pub fn WithActionForm() -> impl IntoView {
    let action = ServerAction::<AddRow>::new();
    let row_count =
        Resource::new(move || action.version().get(), |_| get_rows());

    view! {
        <h3>Using <code>"<ActionForm/>"</code></h3>
        <p>
            <code>"<ActionForm/>"</code>
            "lets you use an HTML "
            <code>"<form>"</code>
            "to call a server function in a way that gracefully degrades."
        </p>
        <ActionForm action>
            <input
                // the `name` of the input corresponds to the argument name
                name="text"
                placeholder="Type something here."
            />
            <button>Submit</button>
        </ActionForm>
        <p>You submitted: {move || format!("{:?}", action.input().get())}</p>
        <p>The result was: {move || format!("{:?}", action.value().get())}</p>
        <Transition>
            archive underaligned: need alignment 4 but have alignment 1
            <p>Total rows: {row_count}</p>
        </Transition>
    }
}

#[server(
    prefix = "/api2",
    endpoint = "custom_path",
    input = GetUrl,
    output = SerdeLite,
)]
#[middleware(crate::middleware::LoggingLayer)]
pub async fn length_of_input(input: String) -> Result<usize, ServerFnError> {
    println!("2. Running server function.");
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(input.len())
}

#[component]
pub fn ServerFnArgumentExample() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = signal(0);

    view! {
        <h3>Custom arguments to the <code>#[server]</code> " macro"</h3>
        <p>This example shows how to specify additional behavior, including:</p>
        <ul>
            <li>Specific server function <strong>paths</strong></li>
            <li>Mixing and matching input and output <strong>encodings</strong></li>
            <li>Adding custom <strong>middleware</strong>on a per-server-fn basis</li>
        </ul>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let length = length_of_input(value).await.unwrap_or(0);
                set_result.set(length);
            });
        }>

            Click to see length
        </button>
        <p>Length is {result}</p>
    }
}

#[server(
    input = Rkyv,
    output = Rkyv
)]
pub async fn rkyv_example(input: String) -> Result<String, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(input.to_ascii_uppercase())
}

#[component]
pub fn RkyvExample() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (input, set_input) = signal(String::new());
    let rkyv_result = Resource::new(move || input.get(), rkyv_example);

    view! {
        <h3>Using <code>rkyv</code>encoding</h3>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            set_input.set(value);
        }>

            Click to capitalize
        </button>
        <p>{input}</p>
        <Transition>{rkyv_result}</Transition>
    }
}

#[component]
pub fn FileUpload() -> impl IntoView {
    #[server(
        input = MultipartFormData,
    )]
    pub async fn file_length(
        data: MultipartData,
    ) -> Result<usize, ServerFnError> {
        let mut data = data.into_inner().unwrap();

        let mut count = 0;
        while let Ok(Some(mut field)) = data.next_field().await {
            println!("\n[NEXT FIELD]\n");
            let name = field.name().unwrap_or_default().to_string();
            println!("  [NAME] {name}");
            while let Ok(Some(chunk)) = field.chunk().await {
                let len = chunk.len();
                count += len;
                println!("      [CHUNK] {len}");
                // in a real server function, you'd do something like saving the file here
            }
        }

        Ok(count)
    }

    let upload_action = Action::new_local(|data: &FormData| {
        file_length(data.clone().into())
    });

    view! {
        <h3>File Upload</h3>
        <p>Uploading files is fairly easy using multipart form data.</p>
        <form on:submit=move |ev: SubmitEvent| {
            ev.prevent_default();
            let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
            let form_data = FormData::new_with_form(&target).unwrap();
            upload_action.dispatch_local(form_data);
        }>
            <input type="file" name="file_to_upload" />
            <input type="submit" />
        </form>
        <p>
            {move || {
                if upload_action.input().read().is_none() && upload_action.value().read().is_none()
                {
                    "Upload a file.".to_string()
                } else if upload_action.pending().get() {
                    "Uploading...".to_string()
                } else if let Some(Ok(value)) = upload_action.value().get() {
                    value.to_string()
                } else {
                    format!("{:?}", upload_action.value().get())
                }
            }}

        </p>
    }
}

#[component]
pub fn FileUploadWithProgress() -> impl IntoView {
    #[cfg(feature = "ssr")]
    mod progress {
        use async_broadcast::{broadcast, Receiver, Sender};
        use dashmap::DashMap;
        use futures::Stream;
        use std::sync::LazyLock;

        struct File {
            total: usize,
            tx: Sender<usize>,
            rx: Receiver<usize>,
        }

        static FILES: LazyLock<DashMap<String, File>> =
            LazyLock::new(DashMap::new);

        pub async fn add_chunk(filename: &str, len: usize) {
            println!("[{filename}]\tadding {len}");
            let mut entry =
                FILES.entry(filename.to_string()).or_insert_with(|| {
                    println!("[{filename}]\tinserting channel");
                    let (tx, rx) = broadcast(1048);
                    File { total: 0, tx, rx }
                });
            entry.total += len;
            let new_total = entry.total;

            let tx = entry.tx.clone();
            drop(entry);

            tx.broadcast(new_total)
                .await
                .expect("couldn't send a message over channel");
        }

        pub fn for_file(filename: &str) -> impl Stream<Item = usize> {
            let entry =
                FILES.entry(filename.to_string()).or_insert_with(|| {
                    println!("[{filename}]\tinserting channel");
                    let (tx, rx) = broadcast(128);
                    File { total: 0, tx, rx }
                });
            entry.rx.clone()
        }
    }

    #[server(
        input = MultipartFormData,
    )]
    pub async fn upload_file(data: MultipartData) -> Result<(), ServerFnError> {
        let mut data = data.into_inner().unwrap();

        while let Ok(Some(mut field)) = data.next_field().await {
            let name =
                field.file_name().expect("no filename on field").to_string();
            while let Ok(Some(chunk)) = field.chunk().await {
                let len = chunk.len();
                println!("[{name}]\t{len}");
                progress::add_chunk(&name, len).await;
            }
        }

        Ok(())
    }

    #[server(output = StreamingText)]
    pub async fn file_progress(
        filename: String,
    ) -> Result<TextStream, ServerFnError> {
        println!("getting progress on {filename}");
        let progress = progress::for_file(&filename);
        let progress = progress.map(|bytes| Ok(format!("{bytes}\n")));
        Ok(TextStream::new(progress))
    }

    let (filename, set_filename) = signal(None);
    let (max, set_max) = signal(None);
    let (current, set_current) = signal(None);
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
        let form_data = FormData::new_with_form(&target).unwrap();
        let file = form_data
            .get("file_to_upload")
            .unchecked_into::<web_sys::File>();
        let filename = file.name();
        let size = file.size() as usize;
        set_filename.set(Some(filename.clone()));
        set_max.set(Some(size));
        set_current.set(None);

        spawn_local(async move {
            let mut progress = file_progress(filename)
                .await
                .expect("couldn't initialize stream")
                .into_inner();
            while let Some(Ok(len)) = progress.next().await {
                let len = len
                    .split('\n')
                    .filter(|n| !n.is_empty())
                    .next_back()
                    .expect(
                        "expected at least one non-empty value from \
                         newline-delimited rows",
                    )
                    .parse::<usize>()
                    .expect("invalid length");
                set_current.set(Some(len));
            }
        });
        spawn_local(async move {
            upload_file(form_data.into())
                .await
                .expect("couldn't upload file");
        });
    };

    view! {
        <h3>File Upload with Progress</h3>
        <p>A file upload with progress can be handled with two separate server functions.</p>
        <aside>See the doc comment on the component for an explanation.</aside>
        <form on:submit=on_submit>
            <input type="file" name="file_to_upload" />
            <input type="submit" />
        </form>
        {move || filename.get().map(|filename| view! { <p>Uploading {filename}</p> })}
        <ShowLet some=max let:max>
            <progress
                max=max
                value=move || current.get().unwrap_or_default()
            ></progress>
        </ShowLet>
    }
}
#[component]
pub fn FileWatcher() -> impl IntoView {
    #[server(input = GetUrl, output = StreamingText)]
    pub async fn watched_files() -> Result<TextStream, ServerFnError> {
        use notify::{
            Config, Error, Event, RecommendedWatcher, RecursiveMode, Watcher,
        };
        use std::path::Path;

        let (tx, rx) = futures::channel::mpsc::unbounded();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, Error>| {
                if let Ok(ev) = res {
                    if let Some(path) = ev.paths.last() {
                        let filename = path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string();
                        _ = tx.unbounded_send(filename); //res);
                    }
                }
            },
            Config::default(),
        )?;
        watcher
            .watch(Path::new("./watched_files"), RecursiveMode::Recursive)?;
        std::mem::forget(watcher);

        Ok(TextStream::from(rx))
    }

    let (files, set_files) = signal(Vec::new());

    Effect::new(move |_| {
        spawn_local(async move {
            while let Some(res) =
                watched_files().await.unwrap().into_inner().next().await
            {
                if let Ok(filename) = res {
                    set_files.update(|n| n.push(filename));
                }
            }
        });
    });

    view! {
        <h3>Watching files and returning a streaming response</h3>
        <p>Files changed since you loaded the page:</p>
        <ul>
            {move || {
                files
                    .get()
                    .into_iter()
                    .map(|file| {
                        view! {
                            <li>
                                <code>{file}</code>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

        </ul>
        <p>
            <em>
                Add or remove some text files in the <code>watched_files</code>
                directory and see the list of changes here.
            </em>
        </p>
    }
}

#[server]
pub async fn ascii_uppercase(text: String) -> Result<String, MyErrors> {
    other_error()?;
    Ok(ascii_uppercase_inner(text)?)
}

pub fn other_error() -> Result<(), String> {
    Ok(())
}

pub fn ascii_uppercase_inner(text: String) -> Result<String, InvalidArgument> {
    if text.len() < 5 {
        Err(InvalidArgument::TooShort)
    } else if text.len() > 15 {
        Err(InvalidArgument::TooLong)
    } else if text.is_ascii() {
        Ok(text.to_ascii_uppercase())
    } else {
        Err(InvalidArgument::NotAscii)
    }
}

#[server]
pub async fn ascii_uppercase_classic(
    text: String,
) -> Result<String, ServerFnError<InvalidArgument>> {
    Ok(ascii_uppercase_inner(text)?)
}

#[derive(
    thiserror::Error,
    Debug,
    Clone,
    Display,
    EnumString,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum InvalidArgument {
    TooShort,
    TooLong,
    NotAscii,
}

#[derive(
    thiserror::Error,
    Debug,
    Clone,
    Display,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum MyErrors {
    InvalidArgument(InvalidArgument),
    ServerFnError(ServerFnErrorErr),
    Other(String),
}

impl From<InvalidArgument> for MyErrors {
    fn from(value: InvalidArgument) -> Self {
        MyErrors::InvalidArgument(value)
    }
}

impl From<String> for MyErrors {
    fn from(value: String) -> Self {
        MyErrors::Other(value)
    }
}

impl FromServerFnError for MyErrors {
    type Encoder = RkyvEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        MyErrors::ServerFnError(value)
    }
}

#[component]
pub fn CustomErrorTypes() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = signal(None);
    let (result_classic, set_result_classic) = signal(None);

    view! {
        <h3>Using custom error types</h3>
        <p>
            "Server functions can use a custom error type that is preserved across the network boundary."
        </p>
        <p>
            "Try typing a message that is between 5 and 15 characters of ASCII text below. Then try breaking \
            the rules!"
        </p>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let data = ascii_uppercase(value.clone()).await;
                let data_classic = ascii_uppercase_classic(value).await;
                set_result.set(Some(data));
                set_result_classic.set(Some(data_classic));
            });
        }>

            "Submit"
        </button>
        <p>{move || format!("{:?}", result.get())}</p>
        <p>{move || format!("{:?}", result_classic.get())}</p>
    }
}

pub struct Toml;

#[derive(Serialize, Deserialize)]
pub struct TomlEncoded<T>(T);

impl ContentType for Toml {
    const CONTENT_TYPE: &'static str = "application/toml";
}

impl FormatType for Toml {
    const FORMAT_TYPE: Format = Format::Text;
}

impl Encoding for Toml {
    const METHOD: Method = Method::POST;
}

impl<T, Request, Err> IntoReq<Toml, Request, Err> for TomlEncoded<T>
where
    Request: ClientReq<Err>,
    T: Serialize,
    Err: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, Err> {
        let data = toml::to_string(&self.0).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Request::try_new_post(path, Toml::CONTENT_TYPE, accepts, data)
    }
}

impl<T, Request, Err> FromReq<Toml, Request, Err> for TomlEncoded<T>
where
    Request: Req<Err> + Send,
    T: DeserializeOwned,
    Err: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, Err> {
        let string_data = req.try_into_string().await?;
        toml::from_str::<T>(&string_data)
            .map(TomlEncoded)
            .map_err(|e| ServerFnErrorErr::Args(e.to_string()).into_app_error())
    }
}

impl<T, Response, Err> IntoRes<Toml, Response, Err> for TomlEncoded<T>
where
    Response: TryRes<Err>,
    T: Serialize + Send,
    Err: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, Err> {
        let data = toml::to_string(&self.0).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Response::try_from_string(Toml::CONTENT_TYPE, data)
    }
}

impl<T, Response, Err> FromRes<Toml, Response, Err> for TomlEncoded<T>
where
    Response: ClientRes<Err> + Send,
    T: DeserializeOwned,
    Err: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, Err> {
        let data = res.try_into_string().await?;
        toml::from_str(&data).map(TomlEncoded).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct WhyNotResult {
    original: String,
    modified: String,
}

#[server(
    input = Toml,
    output = Toml,
    custom = TomlEncoded
)]
pub async fn why_not(
    original: String,
    addition: String,
) -> Result<TomlEncoded<WhyNotResult>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(TomlEncoded(WhyNotResult {
        modified: format!("{original}{addition}"),
        original,
    }))
}

#[component]
pub fn CustomEncoding() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = signal("foo".to_string());

    view! {
        <h3>Custom encodings</h3>
        <p>
            "This example creates a custom encoding that sends server fn data using TOML. Why? Well... why not?"
        </p>
        <input node_ref=input_ref placeholder="Type something here." />
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let new_value = why_not(value, ", but in TOML!!!".to_string()).await.unwrap();
                set_result.set(new_value.0.modified);
            });
        }>

            Submit
        </button>
        <p>{result}</p>
    }
}

#[component]
pub fn CustomClientExample() -> impl IntoView {
    // Define a type for our client.
    pub struct CustomClient;

    impl<E, IS, OS> Client<E, IS, OS> for CustomClient
    where
        E: FromServerFnError,
        IS: FromServerFnError,
        OS: FromServerFnError,
    {
        type Request = BrowserRequest;
        type Response = BrowserResponse;

        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, E>> + Send {
            let headers = req.headers();
            headers.append("X-Custom-Header", "foobar");
            <BrowserClient as Client<E, IS, OS>>::send(req)
        }

        fn open_websocket(
            path: &str,
        ) -> impl Future<
            Output = Result<
                (
                    impl Stream<
                            Item = Result<server_fn::Bytes, server_fn::Bytes>,
                        > + Send
                        + 'static,
                    impl Sink<server_fn::Bytes> + Send + 'static,
                ),
                E,
            >,
        > + Send {
            <BrowserClient as Client<E, IS, OS>>::open_websocket(path)
        }

        fn spawn(future: impl Future<Output = ()> + Send + 'static) {
            <BrowserClient as Client<E, IS, OS>>::spawn(future)
        }
    }

    #[server(client = CustomClient)]
    pub async fn fn_with_custom_client() -> Result<(), ServerFnError> {
        use http::header::HeaderMap;
        use leptos_axum::extract;

        let headers: HeaderMap = extract().await?;
        let custom_header = headers.get("X-Custom-Header");
        println!("X-Custom-Header = {custom_header:?}");
        Ok(())
    }

    view! {
        <h3>Custom clients</h3>
        <p>
            You can define a custom server function client to do something like adding a header to every request.
        </p>
        <p>
            Check the network request in your browser devtools to see how this client adds a custom header.
        </p>
        <button on:click=|_| spawn_local(async {
            fn_with_custom_client().await.unwrap()
        })>Click me</button>
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PostcardData {
    name: String,
    age: u32,
    hobbies: Vec<String>,
}

#[server(input = Postcard, output = Postcard)]
pub async fn postcard_example(
    data: PostcardData,
) -> Result<PostcardData, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let mut modified_data = data.clone();
    modified_data.age += 1;
    modified_data.hobbies.push("Rust programming".to_string());

    Ok(modified_data)
}

#[component]
pub fn PostcardExample() -> impl IntoView {
    let (input, set_input) = signal(PostcardData {
        name: "Alice".to_string(),
        age: 30,
        hobbies: vec!["reading".to_string(), "hiking".to_string()],
    });

    let postcard_result = Resource::new(
        move || input.get(),
        |data| async move { postcard_example(data).await },
    );

    view! {
        <h3>Using <code>postcard</code>encoding</h3>
        <p>"This example demonstrates using Postcard for efficient binary serialization."</p>
        <button on:click=move |_| {
            set_input
                .update(|data| {
                    data.age += 1;
                });
        }>"Increment Age"</button>
        <p>"Input: " {move || format!("{:?}", input.get())}</p>
        <Transition>
            <p>"Result: " {move || postcard_result.get().map(|r| format!("{:?}", r))}</p>
        </Transition>
    }
}
