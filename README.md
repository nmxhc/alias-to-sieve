This script converts an alias file to a sieve script for [stalwart-mail](https://stalw.art/).

## Usage
Given an alias file `testdata/simple.aliases` that contains lines of redirects of the form localpart with optional `@fqdn` followed by a space followed by a list (space or comma+space separated) list of destinations that consist of a localpart and optionally an `@fqdn`.
If you don't define a fqdn along any of the addresses, the default domain from your commandline input will be appended.

An example using the testdata directory of this repository:
```shell
$ ./alias_to_sieve testdata/simple.aliases example.com
```
```sieve
require ["variables", "copy", "vnd.stalwart.expressions", "envelope", "editheader"];

let "i" "0";
while "i < count(envelope.to)" {
  let "redirected" "false";
  if eval "eq_ignore_case(envelope.to[i], 'admin@example.com')" {
    addheader "Delivered-To" "admin@example.com";
    redirect :copy "me@example.org";

    deleteheader :index 1 :is "Delivered-To" "admin@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'postmaster@example.com')" {
    addheader "Delivered-To" "postmaster@example.com";
    redirect :copy "me@example.org";

    deleteheader :index 1 :is "Delivered-To" "postmaster@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'root@example.com')" {
    addheader "Delivered-To" "root@example.com";
    redirect :copy "me@example.org";

    deleteheader :index 1 :is "Delivered-To" "root@example.com";
    let "redirected" "true";
  }
  if eval "eq_ignore_case(envelope.to[i], 'sudo@example.com')" {
    addheader "Delivered-To" "sudo@example.com";
    redirect :copy "me@example.org";

    deleteheader :index 1 :is "Delivered-To" "sudo@example.com";
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

If you have multiple domains with multiple alias files, pass them all in one run: `$ ./alias_to_sieve simple.aliases example.com other.aliases example.org`.

## Limitations
You cannot use apostrophes (') in any mail addresses although allowed by [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322) since they would break termination of strings in sieve.

This parser is not designed with security in mind. While the above gives some basic protection against code injection, I have no idea whether sieve has other pitfalls that might allow them.

This is my first rust project, consume the code with care.

The generated code is specific to stalwart-mail and contains non-standard sieve features.


## Configure stalwart-mail to use it

1. Until [56450c6](https://github.com/stalwartlabs/sieve/commit/56450c6ccdf76f1de95931db24896599159efc53) is included in stalwart-mail by default, you need to patch stalwart's `Cargo.lock` file to include it and build it yourself: 
```diff
diff --git a/Cargo.lock b/Cargo.lock
index be36759b..b4316639 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -6404,8 +6404,7 @@ checksum = "0fda2ff0d084019ba4d7c6f371c95d8fd75ce3524c3cb8fb653a3023f6323e64"
 [[package]]
 name = "sieve-rs"
 version = "0.6.0"
-source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "15ac54053752c25a0e545dd1953de716abcc80b12cfe0b6c2f2c1c73759d4f45"
+source = "git+https://github.com/stalwartlabs/sieve.git#56450c6ccdf76f1de95931db24896599159efc53"
 dependencies = [
  "ahash 0.8.11",
  "bincode",
diff --git a/Cargo.toml b/Cargo.toml
index f055474f..2b64c9ac 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -63,3 +63,7 @@ incremental = false
 debug-assertions = false
 overflow-checks = false
 rpath = false
+
+
+[patch.crates-io]
+sieve-rs = { git = 'https://github.com/stalwartlabs/sieve.git' }
```
2. Run this program and save its outputs to a file (e.g. `/tmp/virt_aliases`). 
3. Include the following in your configuration TOML: 
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
4. Create the local domains in the web-admin.
5. Create a catch-all account. It does not receive mail that is directed towards redirects but otherwise works as a catch-all account.
