use shuttle_persist::PersistInstance;

#[shuttle_runtime::main]
async fn main(#[shuttle_persist::Persist] persist: PersistInstance) -> shuttle_axum::ShuttleAxum {
    Ok(warren_cmp::app(persist).into())
}
