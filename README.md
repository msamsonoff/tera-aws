# tera-aws

`tera-aws` extends the [Tera](https://tera.netlify.app/) template language with AWS-related functions.

## Command-line

```
tera-aws [OPTIONS] <TEMPLATE> <OUTPUT>
```

*   `-h`, `--help`  
    Prints help information

*   `-v`, `--version`  
    Prints version information

*   `-r`, `--region`  
    Overrides any region value set by environment variable or configuration file

*   `-d`, `--template-dir`  
    The directory containing the Tera templates

*   `<TEMPLATE>`  
    The path of the root template

*   `<OUTPUT>`  
    The path to the output file

```
tera-aws ./templates/knot.conf /etc/knot/knot.conf
```

## Functions

### imds

This function retrieves string data from IMDSv2.

```
export INSTANCE_ID="{{ imds(path="latest/meta-data/instance-id") }}"
```

#### Parameters

*   `path`  
    string, required  
    The metadata path.

### ec2_describe_tags

This function gets tags for EC2 resources.

```
{% set tags = ec2_describe_tags(filters=["Name=resource-id,Values=" ~ imds(path="/latest/meta-data/instance-id")]) %}
    {% if tag.key == "Name" %}
instance_name={{ tag.value }}
    {% endif %}
{% endfor %}
```

#### Parameters

*   `filters`  
    string, optional  
    Set the filters for the `describe-tags` API call.
    These strings must match the format used by the AWS CLI.
    For example:

            Name=resource-id,Values=i-0123456789
            Name=tag:Name,Values=database*
            Name=tag:aws:cloudformation:stack-name,Values=stack-one,stack-two

### secretsmanager_get_secret_value

This function gets the value of a string secret from AWS Secrets Manager.
(Binary secrets are not supported.)

```
postgres:5432:postgres:user:{{ secretsmanager_get_secret_value(secret_id="PostgresPassword") }}
```

#### Parameters

*   `secret_id`  
    string, required  
    The name or ARN of the secret.

### secretsmanager_get_secret_value_json

This function is identical to `secretsmanager_get_secret_value` above but it parses the response as a JSON string and evaluates to a JSON value.
This will usually require that you assign the result to a variable using `set` or `set_global`.

```
{% set key = secretsmanager_get_secret_value_json(secret_id="KnotKey") %}
key:
- id: example.org
  algorithm: {{ key.algorithm }}
  secret: {{ key.secret }}
```

#### Parameters

*   `secret_id`  
    string, required  
    The name or ARN of the secret.

## Known Issues

*   `ec2_describe_tags` is limited to a single page of results.
    I am waiting for further information on paginators in the AWS SDK for Rust before implementing anything.
    See [aws-sdk-rust issue #47](https://github.com/awslabs/aws-sdk-rust/issues/47).
