use mdns_sd::{ServiceDaemon, ServiceInfo};
use log::info;

const SERVICE_TYPE: &str = "_moyuguard._tcp.local.";

pub struct MdnsServer {
    daemon: ServiceDaemon,
    service_fullname: String,
}

impl MdnsServer {
    pub fn new(port: u16) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let daemon = ServiceDaemon::new()?;
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &hostname,
            &format!("{}.", hostname),
            "",
            port,
            None,
        )?;

        let fullname = service.get_fullname().to_string();
        daemon.register(service)?;
        info!("mDNS service registered: {} on port {}", fullname, port);

        Ok(Self {
            daemon,
            service_fullname: fullname,
        })
    }

    pub fn shutdown(&self) {
        let _ = self.daemon.unregister(&self.service_fullname);
        let _ = self.daemon.shutdown();
    }
}

impl Drop for MdnsServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
