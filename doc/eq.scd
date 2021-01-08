eq(1)

# NAME

eq - query Elasticsearch

# SYNOPSIS

*eq [OPTIONS]*

# DESCRIPTION

*eq* is a simple command line interface for Elasticsearch queries.

*eq*'s defaults are set up for querying log messages ingested into
Elasticsearch from Filebeat. The default index pattern is "filebeat-\*", only
"\_source.message" fields are printed, and results are sorted by "\@timestamp".
The default query is the query string "\*" which will return all results,
you'll likely want to limit this using _--query_, for example "--query
'agent.hostname: my-host'".

To query other types of documents use the _--index_, _--json_, and _--sort_
options.

# OPTIONS

*-a*, *--address* _ADDRESS_
	The address of the Elasticsearch server to query. The default is
	_http://localhost:9200_. Can be set by the environment variable
	*$ES_ADDRESS*.

*-b*, *--batch-size* _NUM_
	Return NUM results per query. The default is _1000_.

*-f*, *--follow*
	Follow results, keep searching for new results until canceled.

*-h*, *--help*
	Prints help information.

*-i*, *--index* _INDEX_
	The index or index pattern to query. The default is _filebeat-\*_.

*-j*, *--json*
	Print hits as newline delimited json objects, including all fields.

	This is intended for use with other command line tools, for example piping
	into *jq*[1] or *gron*[2] for additional filtering and formatting as desired.

*-l*, *--limit* _NUM_
	Return NUM results, 0 means no limit. The default is _10000_.

*-n*, *--no-certificate-validation*
	Do not validate the SSL/TLS certificate of the server.

*-p*, *--password* _PASSWORD_
	The Elasticsearch password to use. Can be set by the environment variable
	*$ES_PASSWORD*.

*-q*, *--query* _QUERY_STRING_
	The query string to search with. The default is _\*_.

	Take a look at the Elasticsearch query string syntax[3] for details.

*-Q*, *--query-dsl* _QUERY_DSL_JSON_
	The query Domain Specific Language (DSL) json to search with. If set, this
	overrides the query string from --query.

	Take a look at the Elasticsearch query DSL[4] documentation
	for details.

*-s*, *--sort* _FIELD:DIRECTION[,...]_
	Comma separated list of field:direction pairs to control sorting of results.
	The default is _@timestamp:asc,\_id:asc_.

*-u*, *--username* _USERNAME_
	The Elasticsearch username to use. Can be set by the environment variable
	*$ES_USERNAME*.

*-v*, *--verbose*
	Print extra information to standard error.

*-V*, *--version*
	Print version information.

# AUTHOR

Maintained by Chris Koehnke <chris@koehnke.xyz>. Source is at
https://github.com/conky5/eq.

# NOTES

1. *jq* - jq is a lightweight and flexible command-line JSON processor.
```
https://stedolan.github.io/jq/
```

2. *gron* - Make JSON greppable!
```
https://github.com/tomnomnom/gron
```

3. Elasticsearch query string syntax
```
https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-query-string-query.html#query-string-syntax
```

4. Elasticsearch query DSL
```
https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html
```
