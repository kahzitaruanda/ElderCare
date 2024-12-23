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
struct CareReport {
    id: u64,
    elder_name: String,
    caregiver_name: String,
    report_details: String,
    timestamp: u64,
    updated_at: Option<u64>,
}

impl Storable for CareReport {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for CareReport {
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

    static REPORTS: RefCell<StableBTreeMap<u64, CareReport, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct CareReportPayload {
    elder_name: String,
    caregiver_name: String,
    report_details: String,
}

#[ic_cdk::query]
fn get_report(id: u64) -> Result<CareReport, String> {
    REPORTS.with(|storage| storage.borrow().get(&id).cloned())
        .ok_or_else(|| format!("Care report with ID {} not found", id))
}

#[ic_cdk::update]
fn add_report(payload: CareReportPayload) -> CareReport {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let report = CareReport {
        id,
        elder_name: payload.elder_name,
        caregiver_name: payload.caregiver_name,
        report_details: payload.report_details,
        timestamp: time(),
        updated_at: None,
    };

    REPORTS.with(|storage| storage.borrow_mut().insert(id, report.clone()));
    report
}

#[ic_cdk::update]
fn update_report(id: u64, payload: CareReportPayload) -> Result<CareReport, String> {
    REPORTS.with(|storage| {
        let mut reports = storage.borrow_mut();
        if let Some(report) = reports.get_mut(&id) {
            report.elder_name = payload.elder_name;
            report.caregiver_name = payload.caregiver_name;
            report.report_details = payload.report_details;
            report.updated_at = Some(time());
            Ok(report.clone())
        } else {
            Err(format!("Care report with ID {} not found", id))
        }
    })
}

#[ic_cdk::update]
fn delete_report(id: u64) -> Result<CareReport, String> {
    REPORTS.with(|storage| storage.borrow_mut().remove(&id))
        .ok_or_else(|| format!("Care report with ID {} not found", id))
}

// Generate candid interface
ic_cdk::export_candid!();
