use email_address_parser::EmailAddress;
use fqdn::FQDN;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub struct AliasFile {
    pub content: io::Lines<io::BufReader<File>>,
    pub default_domain: FQDN,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AliasEmailAddress(EmailAddress);

impl AliasEmailAddress {
    /// Create an `AliasEmailAddress` from some alias entry.
    /// Return parameter for complete mail addresses and append the default domain for local parts.
    pub fn new(
        alias_entry: &str,
        default_domain: &FQDN,
    ) -> Result<AliasEmailAddress, Box<dyn Error>> {
        let mut addr = alias_entry.trim().to_string();
        addr = addr.replace(',', "");

        // The domain already fails on instantiation of the FQDN type if it contains an apostrophe.
        if addr.contains('\'') {
            return Err(format!(
                "Mailaddress {addr} contains an apostrophe which breaks the script generation."
            )
            .into());
        }

        if addr.contains('@') {
            return Ok(AliasEmailAddress(
                EmailAddress::parse(&addr, None).ok_or::<Box<dyn Error>>(
                    String::from("Mailaddress {addr} not parsable.").into(),
                )?,
            ));
        }
        let unsortable_mail = EmailAddress::new(&addr, &default_domain.to_string(), None)?;
        Ok(AliasEmailAddress(unsortable_mail))
    }
}

impl PartialOrd for AliasEmailAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.to_string().cmp(&other.0.to_string()))
    }
}

impl Ord for AliasEmailAddress {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_string().cmp(&other.0.to_string())
    }
}

pub type AliasMap = BTreeMap<AliasEmailAddress, Vec<AliasEmailAddress>>;

/// Read a virtual alias file <https://www.postfix.org/virtual.5.html>
/// and convert it to a map of destination addresses to a list of their final forwarding addresses.
pub fn parse_alias_to_map(alias_files: Vec<AliasFile>) -> Result<AliasMap, Box<dyn Error>> {
    // File must exist in the current path
    let mut redirect_map: AliasMap = AliasMap::new();
    let mut destinations: Vec<AliasEmailAddress> = Vec::new();

    // Extract all pairs (destination to redirect addresses) from the alias files
    for alias_file in alias_files {
        for line in alias_file.content {
            // Ignore comments in the alias file
            let line = line?;
            let line = String::from(line.split_at(line.find('#').unwrap_or(line.len())).0);
            let destination = line.split_at(line.find(char::is_whitespace).unwrap_or(0)).0;

            if destination.is_empty() {
                continue;
            }

            let redirects: Vec<AliasEmailAddress> = line
                .split_at(line.find(char::is_whitespace).unwrap_or(0))
                .1
                .split(' ')
                .filter(|address| !address.trim().to_string().replace(',', "").is_empty())
                .map(|addr| AliasEmailAddress::new(addr, &alias_file.default_domain))
                .collect::<Result<Vec<_>, _>>()?;

            if redirects.is_empty() {
                continue;
            }
            destinations.push(AliasEmailAddress::new(
                destination,
                &alias_file.default_domain,
            )?);
            redirect_map.insert(
                AliasEmailAddress::new(destination, &alias_file.default_domain)?,
                redirects,
            );
        }
    }

    // Replace redirects that are again forwarded elsewhere by that.
    // Break after depth max_iterations and assume infinite recursion afterwards.
    let mut changed = true;
    let mut iterations = 0;
    let max_iterations = 100;
    while changed && iterations < max_iterations {
        changed = false;
        iterations += 1;
        let mut all_new_redirects: AliasMap = AliasMap::new();
        for destination in &destinations {
            for forward_to in redirect_map.get(destination).unwrap() {
                if let Some(new_redirects) = redirect_map.get(forward_to) {
                    changed = true;
                    all_new_redirects
                        .entry(destination.clone())
                        .or_insert(redirect_map.get(destination).unwrap().clone())
                        .retain(|dest| *dest != *forward_to);
                    all_new_redirects
                        .entry(destination.clone())
                        .and_modify(|d| d.extend(new_redirects.iter().cloned()));
                }
            }
        }
        for (destination, new_redirect) in all_new_redirects {
            *redirect_map.get_mut(&destination).unwrap() = new_redirect;
        }
    }
    if iterations == max_iterations {
        return Err(format!("Possibly infinite recursion detected in parse_alias_map. Did not terminate after {max_iterations} rounds.").into());
    }
    Ok(redirect_map)
}

// The output is wrapped in a Result to allow matching on errors.
// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Generate a Sieve script <https://en.wikipedia.org/wiki/Sieve_(mail_filtering_language)>
/// from a map of destination addresses to a list of their forwarding addresses.
///
/// Addresses are sorted according to the order on `OrdEmailAddress`.
pub fn generate_sieve_script(redirects: AliasMap) -> String {
    let mut script: String =
        "require [\"variables\", \"copy\", \"vnd.stalwart.expressions\", \"envelope\", \"editheader\"];

let \"i\" \"0\";
while \"i < count(envelope.to)\" {
  let \"redirected\" \"false\";
"
        .to_string();
    for (redirect, mut destinations) in redirects {
        script += format!(
            // inspired by https://github.com/stalwartlabs/mail-server/issues/916#issuecomment-2474844389
            "  if eval \"eq_ignore_case(envelope.to[i], '{}')\" {{
    addheader \"Delivered-To\" \"{}\";
{}
    deleteheader :index 1 :is \"Delivered-To\" \"{}\";
    let \"redirected\" \"true\";
  }}
",
            redirect.0,
            redirect.0,
            {
                let mut subscript: String = String::new();
                destinations.sort();
                for destination in destinations.iter() {
                    subscript += format!("    redirect :copy \"{}\";\n", destination.0).as_str();
                }
                subscript
            },
            redirect.0
        )
        .as_str();
    }
    script += "  if eval \"!redirected\" {
    let \"destination\" \"envelope.to[i]\";
    redirect :copy \"${destination}\";
  }
";
    script += "  let \"i\" \"i+1\";\n";
    script += "}
discard;";
    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn recursion_detection() {
        let result = parse_alias_to_map(vec![AliasFile {
            content: read_lines("testdata/infiniterec.aliases").unwrap(),
            default_domain: FQDN::from_str("example.com").unwrap(),
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn apostrophe_destination_detection() {
        let result = parse_alias_to_map(vec![AliasFile {
            content: read_lines("testdata/apostrophe_destination.aliases").unwrap(),
            default_domain: FQDN::from_str("example.com").unwrap(),
        }]);
        assert!(result.is_err());
    }
    #[test]
    fn apostrophe_redirect_detection() {
        let result = parse_alias_to_map(vec![AliasFile {
            content: read_lines("testdata/apostrophe_redirect.aliases").unwrap(),
            default_domain: FQDN::from_str("example.com").unwrap(),
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn basic_parsing() {
        let result = parse_alias_to_map(vec![AliasFile {
            content: read_lines("testdata/simple.aliases").unwrap(),
            default_domain: FQDN::from_str("example.com").unwrap(),
        }])
        .unwrap();
        assert_eq!(result.len(), 4);

        for redirects in result.iter() {
            assert_eq!(redirects.1[0].0.to_string(), "me@example.org");
        }
    }
}
