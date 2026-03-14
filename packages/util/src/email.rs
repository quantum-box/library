use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_sesv2::types::{
    Body, Content, Destination, EmailContent, Message,
};
use aws_sdk_sesv2::Client;
use mockall::automock;

#[automock]
#[async_trait::async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_email(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct AwsDriverImpl {
    config: SdkConfig,

    ses_client: Client,
}

impl AwsDriverImpl {
    pub async fn new() -> anyhow::Result<Self> {
        let config =
            aws_config::load_defaults(BehaviorVersion::latest()).await;
        let ses_client = Client::new(&config);
        Ok(Self { config, ses_client })
    }

    pub fn config(&self) -> SdkConfig {
        self.config.clone()
    }
}

#[async_trait::async_trait]
impl EmailSender for AwsDriverImpl {
    async fn send_email(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        // let resp = self
        //     .ses_client
        //     .list_contacts()
        //     .contact_list_name("")
        //     .send()
        //     .await?;

        // let contacts = resp.contacts().unwrap_or_default();

        //         let cs: String = contacts
        //             .iter()
        //             .map(|i| i.email_address().unwrap_or_default())
        //             .collect();

        let dest = Destination::builder().to_addresses(to).build();

        let subject =
            Content::builder().data(subject).charset("UTF-8").build()?;

        let body =
            Content::builder().data(body).charset("UTF-8").build()?;
        let body = Body::builder().text(body).build();

        let email = Message::builder().subject(subject).body(body).build();
        let email = EmailContent::builder().simple(email).build();

        self.ses_client
            .send_email()
            .from_email_address(from)
            .destination(dest)
            .content(email)
            .send()
            .await?;
        Ok(())
    }
}
