This script converts an alias file to a sieve script for [stalwart-mail](https://stalw.art/).

All local-parts are considered to be case-insensitive.

## Usage
Given an alias file [`testdata/example.aliases`](testdata/example.aliases) that contains lines of redirects of the form local-part with optional `@fqdn` followed by a space followed by a list (space or comma+space separated) list of destinations that consist of a local-part and optionally an `@fqdn`.
If you don't define an fqdn along any of the addresses, the default domain from your commandline input will be appended.

An example using the testdata directory of this repository:
```shell
$ ./alias_to_sieve testdata/example.aliases example.com
```
```sieve
require ["variables", "copy", "vnd.stalwart.expressions", "envelope", "editheader"];

let "i" "0";
while "i < count(envelope.to)" {
  let "redirected" "false";
  if eval "eq_ignore_case(envelope.to[i], 'cali@example.com')" {
    addheader "Delivered-To" "cali@example.com";
    redirect :copy "camilia@example.com";

    deleteheader :index 1 :is "Delivered-To" "cali@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'camila@example.com')" {
    addheader "Delivered-To" "camila@example.com";
    redirect :copy "camila@example.edu";

    deleteheader :index 1 :is "Delivered-To" "camila@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'jaiden@example.com')" {
    addheader "Delivered-To" "jaiden@example.com";
    redirect :copy "jaiden@example.edu";

    deleteheader :index 1 :is "Delivered-To" "jaiden@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'priscilla@example.com')" {
    addheader "Delivered-To" "priscilla@example.com";
    redirect :copy "baldwin@example.org";

    deleteheader :index 1 :is "Delivered-To" "priscilla@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'root@example.com')" {
    addheader "Delivered-To" "root@example.com";
    redirect :copy "baldwin@example.org";
    redirect :copy "jaiden@example.edu";

    deleteheader :index 1 :is "Delivered-To" "root@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'webteam@example.com')" {
    addheader "Delivered-To" "webteam@example.com";
    redirect :copy "baldwin@example.org";
    redirect :copy "camilia@example.com";
    redirect :copy "jaiden@example.edu";

    deleteheader :index 1 :is "Delivered-To" "webteam@example.com";
    let "redirected" "true";
  }
  if eval "!redirected" {
    let "destination" "envelope.to[i]";
    redirect :copy "${destination}";
  }
  let "i" "i+1";
}
discard;
```

If you have multiple domains with multiple alias files, pass them all in one run: `$ ./alias_to_sieve simple.aliases example.com example.aliases example.org`.

## Limitations
You cannot use apostrophes (') in any mail addresses although allowed by [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322) since they would break termination of strings in sieve.

This parser is not designed with security in mind. While the above gives some basic protection against code injection, I have no idea whether sieve has other pitfalls that might allow them.

This is my first rust project, consume the code with care.

The generated code is specific to stalwart-mail and contains non-standard sieve features.


## Configure stalwart-mail to use it
You need to use a stalwart version of at least 0.12.0. 

1. Run this program and save its outputs to a file (e.g. `/tmp/virt_aliases`). 
2. Include the following in your configuration TOML: 
```toml
[config]
 # We here define what comes from the TOML-file and especially add "sieve.trusted.*" to the default ones
 # because only TOML-based keys may use macros to load files from disk.
 # We want this to be able to load our sieve-script for mail forwarding.
 # See https://stalw.art/docs/configuration/overview/#local-and-database-settings for more details.
local-keys = [
    …
    "sieve.trusted.*",
]

[session.data]
script = "'redirects'"

[sieve.trusted]
return-path = "sender"

 # should be at least max-recipients times the length of your largest alias forwarding list.
[sieve.trusted.limits]
out-messages = 500
redirects = 500

[sieve.trusted.scripts.redirects]
contents = "%{file:/tmp/virt_aliases}%"

[[sieve.trusted.sign]]
if = "is_local_domain('', sender_domain)"
then = "['rsa-' + sender_domain, 'ed25519-' + sender_domain]"

[[sieve.trusted.sign]]
else = false

 # Otherwise stalwart rejects mail for redirects because it does not know the recipient account. 
[session.rcpt]
catch-all = true

```
3. Create the local domains in the web-admin.
4. Create a catch-all account. It does not receive mail that is directed towards redirects but otherwise works as a catch-all account.
