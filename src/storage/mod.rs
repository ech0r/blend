mod sled_storage;

pub use sled_storage::SledStorage;

pub fn init() -> Result<SledStorage, Box<dyn std::error::Error>> {
    let storage = SledStorage::new()?;
    
    // Initialize with default clients
    create_default_clients(&storage)?;
    
    Ok(storage)
}

// Create some default clients for the application
fn create_default_clients(storage: &SledStorage) -> Result<(), Box<dyn std::error::Error>> {
    use crate::models::Client;
    use uuid::Uuid;
    
    // Check if we already have clients
    let existing_clients = storage.get_all_clients()?;
    if !existing_clients.is_empty() {
        // Clients already exist, no need to create defaults
        return Ok(());
    }
    
    // Create 10 default clients
    let default_clients = vec![
        "Acme Corporation",
        "Globex Industries",
        "Stark Enterprises",
        "Wayne Enterprises",
        "Umbrella Corporation",
        "Cyberdyne Systems",
        "Oscorp Industries",
        "Massive Dynamic",
        "Soylent Corp",
        "Initech",
    ];
    
    // Add each client to storage
    for name in default_clients {
        let client = Client {
            id: Uuid::new_v4(),
            name: name.to_string(),
        };
        
        storage.save_client(&client)?;
        log::info!("Created default client: {}", name);
    }
    
    log::info!("Default clients initialized");
    Ok(())
}
