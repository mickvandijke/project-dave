use autonomi::{Amount, QuoteHash, RewardsAddress};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

pub type OrderID = u16;
pub type Payment = (QuoteHash, RewardsAddress, Amount);

#[derive(Debug, Error)]
pub enum PaymentOrderError {
    #[error("Order not found: {0}")]
    OrderNotFound(OrderID),
    #[allow(dead_code)]
    #[error("Failed to serialize payment order: {0}")]
    SerializationError(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OrderMessage {
    Cancelled,
    Completed,
    KeepAlive,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOrder {
    pub id: OrderID,
    pub payments: Vec<Payment>,
    #[serde(skip)]
    confirmation_sender: Sender<OrderMessage>,
}

#[allow(dead_code)]
impl PaymentOrder {
    fn new(payments: Vec<Payment>, confirmation_sender: Sender<OrderMessage>) -> Self {
        Self {
            id: Self::generate_id(),
            payments,
            confirmation_sender,
        }
    }

    fn generate_id() -> OrderID {
        rand::thread_rng().gen::<u16>()
    }

    fn to_json(&self) -> Result<String, PaymentOrderError> {
        let json = serde_json::to_string(self)
            .map_err(|e| PaymentOrderError::SerializationError(e.to_string()))?;
        tracing::debug!("PaymentOrder JSON: {}", json);
        Ok(json)
    }
}

#[derive(Default)]
pub struct PaymentOrderManager {
    orders: Mutex<HashMap<OrderID, PaymentOrder>>,
}

impl PaymentOrderManager {
    #[allow(dead_code)]
    pub async fn create_order(
        &self,
        payments: Vec<Payment>,
    ) -> (PaymentOrder, Receiver<OrderMessage>) {
        const PAYMENT_CHANNEL_SIZE: usize = 128;
        let (sender, receiver) = channel(PAYMENT_CHANNEL_SIZE);

        let order = PaymentOrder::new(payments, sender);

        let mut orders = self.orders.lock().await;

        orders.insert(order.id, order.clone());

        (order, receiver)
    }

    pub async fn send_order_message(
        &self,
        id: OrderID,
        message: OrderMessage,
    ) -> Result<(), PaymentOrderError> {
        let mut orders = self.orders.lock().await;

        let order = orders
            .get_mut(&id)
            .ok_or(PaymentOrderError::OrderNotFound(id))?;

        let _ = order.confirmation_sender.send(message).await;
        Ok(())
    }

    pub async fn confirm_payment(&self, id: OrderID) -> Result<(), PaymentOrderError> {
        self.send_order_message(id, OrderMessage::Completed).await
    }
}
