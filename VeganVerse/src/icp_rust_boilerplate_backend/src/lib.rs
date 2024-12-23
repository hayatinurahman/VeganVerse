#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct VeganProduct {
    id: u64,
    name: String,
    description: String,
    price: u64,
    seller: String,
    availability: bool,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for VeganProduct {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for VeganProduct {
    const MAX_SIZE: u32 = 2048;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, VeganProduct, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct ProductPayload {
    name: String,
    description: String,
    price: u64,
    seller: String,
}

#[ic_cdk::query]
fn get_product(id: u64) -> Result<VeganProduct, String> {
    STORAGE.with(|storage| storage.borrow().get(&id).cloned())
        .ok_or_else(|| format!("Product with ID {} not found", id))
}

#[ic_cdk::update]
fn add_product(payload: ProductPayload) -> VeganProduct {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let product = VeganProduct {
        id,
        name: payload.name,
        description: payload.description,
        price: payload.price,
        seller: payload.seller,
        availability: true,
        created_at: time(),
        updated_at: None,
    };

    STORAGE.with(|storage| storage.borrow_mut().insert(id, product.clone()));
    product
}

#[ic_cdk::update]
fn update_product(id: u64, payload: ProductPayload) -> Result<VeganProduct, String> {
    STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(product) = storage.get_mut(&id) {
            product.name = payload.name;
            product.description = payload.description;
            product.price = payload.price;
            product.seller = payload.seller;
            product.updated_at = Some(time());
            Ok(product.clone())
        } else {
            Err(format!("Product with ID {} not found", id))
        }
    })
}

#[ic_cdk::update]
fn delete_product(id: u64) -> Result<VeganProduct, String> {
    STORAGE.with(|storage| storage.borrow_mut().remove(&id))
        .ok_or_else(|| format!("Product with ID {} not found", id))
}

#[ic_cdk::update]
fn toggle_availability(id: u64) -> Result<VeganProduct, String> {
    STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(product) = storage.get_mut(&id) {
            product.availability = !product.availability;
            product.updated_at = Some(time());
            Ok(product.clone())
        } else {
            Err(format!("Product with ID {} not found", id))
        }
    })
}

// Generate candid interface
ic_cdk::export_candid!();
