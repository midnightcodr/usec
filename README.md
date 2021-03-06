# usec

This is a Rust module with the ability of calculating US stock exchange calendar with full and half-day holidays. I've borrowed code heavily from [https://github.com/xemwebe/cal-calc](https://github.com/xemwebe/cal-calc), special thanks to https://github.com/xemwebe


# Motivation
I've developped simular applications in the past using:
1. database that stores holiday information
2. dotenv file with holiday information as env variable

I was not satisfied with neither approach due to the requirement of having to insert new db records or updating the dotenv file year after year, that's why I came to this rule-based solution.


# Use case
1. Use directly as a rust module in other rust applications that depends on the US Stock exchange calendar information
2. Build a micro-service based on this module, using popular rust web frameworks such as actix-web to provide service to any programs that supports http requests, example usage of the service is to run certain business scripts on a trading date


# Example run
```bash
cargo run --example show_year 2022
```

```bash
# just for the fun, supply JSON formatted env variable to add 3/3/2022 into the rules set
ADDITIONAL_RULES='[{"SingularDay": "2022-03-03"}]' cargo run --example show_year 2022
```