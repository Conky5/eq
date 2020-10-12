use elasticsearch::{
    auth::Credentials,
    cert::CertificateValidation,
    http::{
        response::Response,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    Elasticsearch, Error, SearchParts,
};
use serde_json::{json, to_string_pretty, Value};
use std::{convert::TryInto, process::exit, thread::sleep, time::Duration};
use structopt::StructOpt;
use url::Url;

#[derive(StructOpt)]
#[structopt(about)]
struct Opt {
    /// The address of the Elasticsearch server to query
    #[structopt(
        short,
        long,
        default_value = "http://localhost:9200",
        env = "ES_ADDRESS"
    )]
    address: String,

    /// The number of results to return per batch
    #[structopt(short, long, default_value = "1000")]
    batch_size: usize,

    /// Follow results, keep searching for new results until canceled
    #[structopt(short, long)]
    follow: bool,

    /// The index to query
    #[structopt(short, long, default_value = "filebeat-*")]
    index: String,

    /// Print hits as newline delimited json objects, including all fields
    #[structopt(short, long)]
    json: bool,

    // 10,000 is arbitrarily chosen to try and have a safe-ish out of the box experience
    /// The limit of results to return, 0 means no limit
    #[structopt(short, long, default_value = "10000")]
    limit: usize,

    /// Do not validate SSL/TLS certificate of server
    #[structopt(short, long)]
    no_certificate_validation: bool,

    /// The Elasticsearch password to use
    #[structopt(short, long, env = "ES_PASSWORD", hide_env_values = true)]
    password: Option<String>,

    /// The query string to search with
    #[structopt(short, long, default_value = "*")]
    query: String,

    /// The query dsl json to search with, overrides --query
    #[structopt(short = "Q", long, default_value = "{}")]
    query_dsl: Value,

    /// key:value pairs separated by commas to control sorting of results
    #[structopt(short, long, default_value = "@timestamp:asc,_id:asc")]
    sort: String,

    /// The Elasticsearch username to use
    #[structopt(short, long, env = "ES_USERNAME")]
    username: Option<String>,

    /// Log extra information to stderr
    #[structopt(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug)]
struct QueryOptions {
    // elasticsearch-rs api options
    body: Value,
    index: String,
    query_string: String,
    size: usize,
    sort: String,

    // eq specific options
    verbose: bool,
    follow: bool,
    limit: usize,
    print_json: bool,
}

impl QueryOptions {
    pub fn get_limit(&self) -> usize {
        // if we're following, don't have a limit
        if self.follow {
            0
        } else {
            self.limit
        }
    }
}

struct SearchResult {
    search_response_body: Value,
}

impl SearchResult {
    pub fn new(search_response_body: Value) -> SearchResult {
        SearchResult {
            search_response_body,
        }
    }

    fn hits(&mut self) -> Vec<Hit> {
        match self.search_response_body["hits"]["hits"].as_array() {
            Some(hits) => {
                let mut hit_objects = Vec::new();
                for hit in hits {
                    hit_objects.push(Hit::new(hit.clone()));
                }
                hit_objects
            }
            None => vec![],
        }
    }

    fn last_sort_field(&mut self) -> Vec<Value> {
        match self.hits().last() {
            Some(hit) => hit.sort(),
            None => vec![],
        }
    }
}

struct Hit {
    value: Value,
}

impl Hit {
    pub fn new(value: Value) -> Hit {
        Hit { value }
    }

    pub fn message(&self) -> &str {
        match self.value["_source"]["message"].as_str() {
            Some(message) => message,
            None => {
                eprintln!("eq: Document does not have a _source.message field, use --json to get all fields.");
                exit(1)
            }
        }
    }

    pub fn json(&self) -> String {
        self.value.to_string()
    }

    pub fn sort(&self) -> Vec<Value> {
        match self.value["sort"].as_array() {
            Some(array) => array.to_vec(),
            None => panic!("No 'sort' in response, cannot use Search After API, aborting."),
        }
    }
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let server = match Url::parse(&opt.address) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("eq: Could not parse url '{}'.", opt.address);
            if opt.verbose {
                eprintln!("eq: Error: {:?}", error);
            }
            exit(1)
        }
    };

    if opt.verbose {
        eprintln!("eq: Using Elasticsearch url '{}'.", server);
    }

    let connection_pool = SingleNodeConnectionPool::new(server);
    let mut transport_builder = TransportBuilder::new(connection_pool);
    match opt.username {
        None => (),
        Some(username) => {
            transport_builder =
                transport_builder.auth(Credentials::Basic(username, opt.password.unwrap()));
        }
    }
    if opt.no_certificate_validation {
        transport_builder = transport_builder.cert_validation(CertificateValidation::None);
    }
    let transport = match transport_builder.build() {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("eq: Could not build transport for Elasticsearch.");
            eprintln!("eq: Error: {:?}", error);
            exit(1)
        }
    };

    let client = Elasticsearch::new(transport);

    let options = QueryOptions {
        index: opt.index.to_string(),
        verbose: opt.verbose,
        size: opt.batch_size,
        query_string: opt.query.to_string(),
        body: json!(opt.query_dsl),
        sort: opt.sort.to_string(),
        print_json: opt.json,
        follow: opt.follow,
        limit: opt.limit,
    };

    // query some logs
    logs(&client, options)
        .await
        .expect("Unable to get logs from Elasticsearch.");
}

async fn logs(client: &Elasticsearch, mut options: QueryOptions) -> Result<usize, Error> {
    // do the first search
    let response = search(&client, &options, vec![]).await;

    // get the result and hit count, print the logs
    let body = response.json::<Value>().await.unwrap();
    let mut result = SearchResult::new(body);
    let hits = result.hits();
    print_logs(options.print_json, &hits);

    // set up the things we'll be modifying as we search
    let mut total_hits = hits.len();
    let mut sort_values = result.last_sort_field();

    // repeat searches until there are no more hits unless we're following
    while !result.hits().is_empty() || options.follow {
        let mut size = options.size;

        // if we have a limit
        if options.get_limit() != 0 {
            // break if we have reached the limit
            if options.get_limit() <= total_hits {
                eprintln!("eq: Limit '{}' reached.", options.get_limit());
                break;
            }

            // slim down the request size if our next search will hit the limit
            if options.get_limit() <= total_hits + options.size {
                size = options.get_limit() - total_hits
            }
        }

        // if we're not following, exit early if the previous result was less
        // than the batch size to avoid an extra query
        if result.hits().len() < options.size && !options.follow {
            if options.verbose {
                eprintln!(
                    "eq: The previous batch of hits was less then the batch size, assuming we are at the end of our results and exiting the search loop."
                );
            }
            break;
        }

        // update the size option if it's been modified
        options.size = size;
        let response = search(&client, &options, sort_values.clone()).await;
        let body = response.json::<Value>().await.unwrap();
        result = SearchResult::new(body);

        total_hits += result.hits().len();
        print_logs(options.print_json, &result.hits());

        // update the sort values if they are not empty, otherwise keep the
        // previous sort values
        if !result.last_sort_field().is_empty() {
            sort_values = result.last_sort_field();
        }

        // sleep if following and did not find new results
        if options.follow && result.hits().is_empty() {
            sleep(Duration::from_secs(5));
        }
    }

    Ok(total_hits)
}

async fn search(
    client: &Elasticsearch,
    options: &QueryOptions,
    search_after: Vec<Value>,
) -> Response {
    // if our limit is smaller than the batch size, use the limit
    let size = if options.size > options.get_limit() && options.get_limit() > 0 {
        options.get_limit()
    } else {
        options.size
    };

    let mut body = options.body.clone();

    // if we have an empty search body, use the query_string
    if body.as_object().unwrap().is_empty() {
        add_to_serde_value(
            &mut body,
            "query",
            json!({
                "query_string": {
                    "query": options.query_string,
                }
            }),
        );
    }

    // if we have sort values, add them to the query body
    if !search_after.is_empty() {
        // modify the query body to include the "search_after" argument
        // https://www.elastic.co/guide/en/elasticsearch/reference/7.6/search-request-body.html#request-body-search-search-after
        add_to_serde_value(&mut body, "search_after", json!(search_after));
    };

    if options.verbose {
        eprintln!("eq: Search options: {:?}", options);
        eprintln!("eq: Search body: {:?}", body.to_string());
    }

    let index = &[options.index.as_str()];
    let sort = &[options.sort.as_str()];

    let response_result = if options.print_json {
        client
            .search(SearchParts::Index(index))
            .size(size.try_into().unwrap())
            .body(body)
            .sort(sort)
            .send()
            .await
    } else {
        // filter down to only the fields we need when we're not logging the
        // full json document
        client
            .search(SearchParts::Index(index))
            .size(size.try_into().unwrap())
            .body(body)
            .sort(sort)
            .filter_path(&[&*"hits.hits._source.message,hits.hits.sort"])
            .send()
            .await
    };

    verify_response(response_result).await
}

fn print_logs(print_json: bool, hits: &[Hit]) {
    for hit in hits {
        if print_json {
            println!("{}", hit.json())
        } else {
            println!("{}", hit.message())
        }
    }
}

async fn verify_response(response_result: Result<Response, Error>) -> Response {
    match response_result {
        Ok(response) => {
            if response.status_code().is_success() {
                response
            } else {
                eprintln!("eq: A query was unsuccessful.");
                eprintln!("eq: response code: {:?}", response.status_code().as_str());
                eprintln!(
                    "eq: response body:\n{}",
                    to_string_pretty(
                        &response
                            .json::<Value>()
                            .await
                            .expect("Could not get response body for failed search.")
                    )
                    .unwrap()
                );
                exit(1)
            }
        }
        Err(error) => {
            eprintln!("eq: A request encountered an error.");
            eprintln!("eq: {:?}", error);
            exit(1)
        }
    }
}

fn add_to_serde_value<S: Into<String>>(
    existing_value: &mut Value,
    key_to_add: S,
    value_to_add: Value,
) {
    let map = existing_value.as_object_mut().unwrap();
    map.insert(key_to_add.into(), value_to_add);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_to_existing_json_test() {
        let mut json: Value = json!({"a": 1});
        add_to_serde_value(&mut json, "b", json!(2));
        assert_eq!(json, json!({"a": 1, "b": 2}));
    }

    #[tokio::test]
    #[ignore]
    async fn elasticsearch_pagination_test() {
        use elasticsearch::{
            indices::IndicesCreateParts, indices::IndicesDeleteParts, params::Refresh, IndexParts,
        };

        let client = Elasticsearch::default();
        let test_index = "eq-testing";
        let test_record_count = 3;

        // delete the testing index in case it already exists
        client
            .indices()
            .delete(IndicesDeleteParts::Index(&[test_index]))
            .send()
            .await
            .expect("Could not delete testing index.");

        // create the testing index
        let index_creation_result = client
            .indices()
            .create(IndicesCreateParts::Index(test_index))
            .send()
            .await;

        // verify we got a successful response
        verify_response(index_creation_result).await;

        // create some testing records
        for i in 0..test_record_count {
            let index_result = client
                .index(IndexParts::Index(test_index))
                .body(json!({
                    "@timestamp": format!("2020-03-1{}T18:11:38.988Z", i),
                    "message": format!("log entry {}", i),
                    "host": "a",
                }))
                .send()
                .await;

            verify_response(index_result).await;
        }
        // add one more record that should not be hit by the searches
        verify_response(
            client
                .index(IndexParts::Index(test_index))
                .body(json!({
                    "@timestamp": format!("2020-03-1{}T18:11:38.988Z", test_record_count),
                    "message": format!("log entry {}", test_record_count),
                    "host": "b"
                }))
                // trigger a refresh on the latest record so we can immediately
                // query for the tests
                .refresh(Refresh::True)
                .send()
                .await,
        )
        .await;

        let query_string_options = QueryOptions {
            index: test_index.to_string(),
            size: 1,
            query_string: "host: a".to_string(),
            body: json!({}),
            sort: "@timestamp:asc,_id:asc".to_string(),
            print_json: false,
            verbose: true,
            follow: false,
            limit: 10,
        };

        // query our test index and see that we saw the full count of records, even with the restricted
        // batch size
        assert_eq!(
            logs(&client, query_string_options.clone()).await.unwrap(),
            test_record_count
        );

        let mut query_dsl_options = query_string_options.clone();

        // change our search to use the query dsl
        query_dsl_options.body = json!({
            "query": {
                "term": {
                    "host" : {
                        "value": "a"
                    }
                }
            }
        });

        // verify the right number of search results from the query dsl
        assert_eq!(
            logs(&client, query_dsl_options).await.unwrap(),
            test_record_count
        );

        let mut no_limit_query_options = query_string_options.clone();
        no_limit_query_options.limit = 0;

        assert_eq!(
            logs(&client, no_limit_query_options).await.unwrap(),
            test_record_count
        );
    }
}
