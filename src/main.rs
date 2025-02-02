use axum::extract::State;
use axum::response::Html;
use axum::{
    routing::{get, post},
    Router,
};
use regex::Regex;
use std::path::Path;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::{env, fs};
use tower_http::trace::TraceLayer;

use tera::{Context, Tera};

#[derive(Clone)]
struct AppState {
    tera: Tera,
    targets: Vec<String>,
    input_directory: String,
    last_target: Arc<Mutex<Option<String>>>,
}

fn get_next_filename(input_directory: &str) -> Option<String> {
    let paths = fs::read_dir(input_directory).unwrap();
    paths
        .filter_map(|p| p.ok())
        .filter_map(|p| p.file_name().into_string().ok())
        .find(|f| f.starts_with("E77") && f.ends_with(".pdf"))
}

fn get_pdf_content(input_directory: &str, filename: &str) -> String {
    let path = Path::new(input_directory)
        .join(filename)
        .to_string_lossy()
        .into_owned();
    println!("running pdftotext on {}", path);
    String::from_utf8(
        std::process::Command::new("pdftotext")
            .arg(path)
            .arg("-")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}

fn parse_month(month: &str) -> u8 {
    match month {
        "Januar" => 1,
        "Februar" => 2,
        "Maerz" => 3,
        "April" => 4,
        "Mai" => 5,
        "Juni" => 6,
        "Juli" => 7,
        "August" => 8,
        "September" => 9,
        "Oktober" => 10,
        "November" => 11,
        "Dezember" => 12,
        _ => panic!("unknown month {}", month),
    }
}

fn extract_date(content: &str) -> Option<String> {
    let regex_date = Regex::new(r"((?<day>\d\d)\.\s*)?(((?<month_numeric>\d\d)\.)|(?<month_text>Januar|Feburar|Maerz|April|Mai|Juni|Juli|August|September|Oktober|November|Dezember)\s*)(?<year>20\d\d)").unwrap();

    if let Some(m) = regex_date.captures(content) {
        println!("found a regex match");
        let day = if let Some(day_str) = m.name("day") {
            day_str.as_str().parse::<u8>().unwrap()
        } else {
            1
        };
        let month = if let Some(month_numeric) = m.name("month_numeric") {
            month_numeric.as_str().parse::<u8>().unwrap()
        } else {
            parse_month(m.name("month_text").unwrap().as_str())
        };
        let year = m.name("year").unwrap().as_str().parse::<u16>().unwrap();
        return Some(format!("{:04}-{:02}-{:02}", year, month, day));
    } else {
        println!("didn't match");
    }
    None
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let mut context = Context::new();

    let next_filename = get_next_filename(&state.input_directory);
    if next_filename.is_none() {
        return Html("done".to_string());
    }
    let next_filename = next_filename.unwrap();

    let pdf_content = get_pdf_content(&state.input_directory, &next_filename);
    let date = extract_date(&pdf_content).unwrap_or("".to_string());

    context.insert("date", &date);
    context.insert("source_filename", &next_filename);
    context.insert("targets", &state.targets);
    context.insert(
        "last_target",
        &state
            .last_target
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(String::new()),
    );
    Html(state.tera.render("index.html", &context).unwrap())
}

async fn pdf(
    State(state): State<AppState>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Vec<u8> {
    let path = Path::new(&state.input_directory)
        .join(filename)
        .to_string_lossy()
        .into_owned();
    fs::read(path).unwrap()
}

fn visit_target(add: bool, input_directory: &str, directory: &Path, result: &mut Vec<String>) {
    let prefix = Path::new(input_directory).canonicalize().unwrap();
    if directory.is_dir() {
        let dir_basename = directory.file_name().unwrap().to_string_lossy();
        if dir_basename.len() == 4 && dir_basename.starts_with("20") {
            return;
        }

        // add to output and recurse
        if add {
            let normalized = directory
                .canonicalize()
                .unwrap()
                .strip_prefix(prefix)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap();
            result.push(normalized);
        }
        for child in directory.read_dir().unwrap() {
            visit_target(true, input_directory, &child.unwrap().path(), result);
        }
    }
}

fn read_targets(input_directory: &str) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    let root = Path::new(input_directory);
    visit_target(false, &input_directory, root, &mut result);
    result
}
use axum::Form;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct RenameForm {
    source: String,
    target: String,
    date: String,
    description: String,
    page: String,
}

async fn rename(State(state): State<AppState>, Form(input): Form<RenameForm>) -> Html<String> {
    if input.date.len() > 4 && !input.description.is_empty() {
        let source_path = Path::new(&state.input_directory)
            .join(&input.source)
            .to_string_lossy()
            .to_string();

        let year = &input.date[..4];
        let target_filename = format!("{} {}.pdf", &input.date, &input.description);
        let target_directory = Path::new(&state.input_directory)
            .join(&input.target)
            .join(year);
        let target_path = target_directory
            .join(target_filename)
            .to_string_lossy()
            .to_string();

        let first_n_pages: Option<u8> = input.page.parse::<u8>().ok();

        if let Ok(mut last_target) = state.last_target.lock() {
            *last_target = Some(input.target);
        }
        std::fs::create_dir_all(&target_directory).unwrap();

        if !std::fs::exists(&target_path).unwrap() {
            if let Some(page_limit) = first_n_pages {
                // only extract the first n pages,
                std::process::Command::new("qpdf")
                    .arg(&source_path)
                    .arg("--pages")
                    .arg(".")
                    .arg(format!("1-{}", page_limit))
                    .arg("--")
                    .arg(&target_path)
                    .status()
                    .unwrap();
                std::process::Command::new("qpdf")
                    .arg(&source_path)
                    .arg("--pages")
                    .arg(".")
                    .arg(format!("{}-z", page_limit + 1))
                    .arg("--")
                    .arg("--replace-input")
                    .status()
                    .unwrap();
            } else {
                // whole file
                println!("Moving {} -> {}", &source_path, &target_path);
                std::fs::rename(source_path, target_path).unwrap();
            }
        } else {
            println!("target {} already exists!", target_path);
        }
    } else {
        println!("You moron forgot something");
    }

    let res = index(State(state));
    res.await
}

#[tokio::main]
async fn main() {
    if let Some(input_directory) = env::args().nth(1) {
        println!("input_directory: {}", input_directory);
        let tera = Tera::new("templates/**/*.html").unwrap();
        let state = AppState {
            tera,
            targets: read_targets(&input_directory),
            input_directory,
            last_target: Arc::new(Mutex::new(None)),
        };

        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        // build our application with a single route
        let app = Router::new()
            .route("/", get(index))
            .route("/", post(rename))
            .route("/pdf/{filename}", get(pdf))
            .layer(TraceLayer::new_for_http())
            .with_state(state);

        // run our app with hyper, listening globally on port 3000
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        println!("Usage: scan-renamer <path>");
        exit(1);
    }
}
