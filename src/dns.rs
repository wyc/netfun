use std::collections::HashMap;

use ipnet::Ipv4Net;
use containers::collections::b_tree::BTree;

type DomainName = String;
type DomainLabel = String;

// "RR"
enum ResourceRecord {
    HostAddress(Ipv4Net),
    MailExchanger(DomainName),
    NameServer(DomainName),
    StartOfAuthority(DomainName),
    CanonicalName(DomainName),
}

type NameServerDb = HashMap<DomainName, ResourceRecord>;

type QName = DomainName;
enum QType {
    MailAgent,
    Glob,
}
enum QClass {
    Glob,
}

trait NameServer {
    fn ord(&self, domain_name: DomainName) -> usize;
    fn findset(&self, domain_name: DomainName) -> Option<Vec<ResourceRecord>>;
    fn relevant(&self, query_type: QType, type_: String) -> bool;
    fn right(&self, name: DomainName, number: usize) -> Option<DomainName>;
    fn copy(&self, rr: &ResourceRecord) -> ResourceRecord;
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {

    }

}
