use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use rocket::{
    get, launch, routes,
    serde::{json::Json, Deserialize, Serialize},
    State,
};

mod ip_pool;
use ip_pool::IPAddrPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    bind_dns: DNSConfig,
    ad_dns: DNSConfig,
    splunk: HTTPConfig,
    ecom: HTTPConfig,
    /// How often (in seconds) to poll the services
    time: usize,
    ip_pools: Vec<IPAddrPool>,
    dev: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSConfig {
    ip: IpAddr,
    domain: String,
    response: Ipv4Addr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPConfig {
    ip: IpAddr,
    port: u16,
    path: String,
    response: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    up: bool,
    last: DateTime<Utc>,
    failure_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatus {
    bind_dns: Status,
    ad_dns: Status,
    smtp: Status,
    pop3: Status,
    ecom: Status,
    splunk: Status,
}

#[get("/status")]
fn hello(status: &State<Arc<Mutex<ServiceStatus>>>) -> Json<ServiceStatus> {
    Json(status.lock().unwrap().clone())
}

#[launch]
fn rocket() -> _ {
    let rocket = rocket::build();
    let default_status = Status {
        up: false,
        last: SystemTime::now().into(),
        failure_reason: "Not Polled Yet".into(),
    };
    let status = Arc::new(Mutex::new(ServiceStatus {
        ecom: default_status.clone(),
        bind_dns: default_status.clone(),
        ad_dns: default_status.clone(),
        smtp: default_status.clone(),
        pop3: default_status.clone(),
        splunk: default_status.clone(),
    }));
    let config = rocket
        .figment()
        .extract_inner("scanner")
        .expect("Failed to read config");
    rocket::tokio::spawn(create_scanner(Arc::clone(&status), config));
    rocket.manage(status).mount("/", routes![hello])
}

fn create_scanner(
    status: Arc<Mutex<ServiceStatus>>,
    config: ScanConfig,
) -> impl Future<Output = ()> + 'static {
    println!("Create Scanner");
    async move {
        let mut interval = rocket::tokio::time::interval(Duration::from_secs(config.time as u64));
        let mut iter = config.ip_pools.iter().cycle();
        loop {
            let ip = iter
                .next()
                .map(|pool| pool.create_ip(config.dev.as_str()))
                .unwrap_or_else(|| IPAddrPool::default_ip(config.dev.as_str())).expect("Failed to get IP");
            //println!("Scan Bind");
            //let bind_dns = scan_dns(&config.bind_dns, ip.ip()).await;
            println!("Scan Ecom");
            let ecom = scan_http(&config.ecom, ip.ip()).await;
            //let smtp = scan_smtp(&config, &ip).await;
            ip.drop().expect("Failed to release IP");
            println!("Update status");
            {
                let mut lock = status.lock().unwrap();
                //lock.bind_dns = bind_dns;
                lock.ecom = ecom;
                //lock.smtp = smtp;
                println!("{:#?}", lock);
            }
            interval.tick().await;
        }
    }
}

async fn scan_dns(config: &DNSConfig, ip: &Ipv4Addr) -> Status {
    todo!()
    //match AsyncResolver::tokio(
    //ResolverConfig::from_parts(
    //None,
    //vec![],
    //NameServerConfigGroup::from_ips_clear(&[config.ip], 53, true),
    //),
    //ResolverOpts::default(),
    //) {
    //Ok(resolver) => match resolver
    //.lookup(
    //config.domain.as_str(),
    //RecordType::A,
    //DnsRequestOptions::default(),
    //)
    //.await
    //{
    //Ok(response) => match response
    //.into_iter()
    //.find(|r| r == &RData::A(config.response))
    //{
    //Some(_) => Status {
    //up: true,
    //last: SystemTime::now().into(),
    //failure_reason: "".into(),
    //},
    //None => Status {
    //up: true,
    //last: SystemTime::now().into(),
    //failure_reason: "Did not return the expected result".into(),
    //},
    //},
    //Err(e) => Status {
    //up: false,
    //last: SystemTime::now().into(),
    //failure_reason: format!("Lookup Failed: {:?}", e),
    //},
    //},
    //Err(e) => Status {
    //up: false,
    //last: SystemTime::now().into(),
    //failure_reason: format!("Resolver Creation Failed: {:?}", e),
    //},
    //}
}

async fn scan_http(config: &HTTPConfig, ip: &Ipv4Addr) -> Status {
    let client = reqwest::ClientBuilder::new()
        .local_address(IpAddr::from(*ip))
        .build()
        .expect("Failed to build client");
    let req = client
        .get(format!("http://{}{}", config.ip, config.path))
        .send()
        .await;
    match req {
        Ok(a) => {
            if a.status().is_success() {
                match a.text().await {
                    Ok(_s) => Status {
                        up: true,
                        last: SystemTime::now().into(),
                        failure_reason: "".into(),
                    },
                    Err(e) => Status {
                        up: true,
                        last: SystemTime::now().into(),
                        failure_reason: format!("Error reading body: {:?}", e),
                    },
                }
            } else {
                Status {
                    up: false,
                    last: SystemTime::now().into(),
                    failure_reason: format!("Get Failed: {:?}", a.status()),
                }
            }
        }
        Err(e) => Status {
            up: false,
            last: SystemTime::now().into(),
            failure_reason: format!("Get Failed: {:?}", e),
        },
    }
}
