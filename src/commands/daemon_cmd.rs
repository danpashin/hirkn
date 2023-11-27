use super::{
    update_cmd::{Command as UpdateCommand, UpdateRequestBuilder},
    CliCommand, GlobalOptions,
};
use crate::config::Config;
use anyhow::{anyhow, Result};
use chrono::Local;
use job_scheduler_ng::{Job, JobScheduler, Schedule};
use std::{sync::Arc, time::Duration};

#[derive(clap::Parser)]
pub(crate) struct Command {
    #[clap(flatten)]
    global_options: GlobalOptions,
}

impl Command {
    async fn construct_update_job(&self, config: Config) -> Result<Job> {
        let schedule: Schedule = match &config.update_schedule {
            Some(schedule) => schedule.parse()?,
            None => return Err(anyhow!("Schedule must be presented for running as daemon!")),
        };

        let command = Arc::new(UpdateCommand::new(self.global_options.clone()));
        let request = Arc::new(UpdateRequestBuilder::new(config).build().await?);

        let job = Job::new(schedule, move || {
            let command = command.clone();
            let request = request.clone();

            tokio::spawn(async move {
                if let Err(error) = command.perform_update(&request).await {
                    log::error!("Error when performing update command: {error:?}");
                }
            });
        });

        Ok(job)
    }
}

#[async_trait]
impl CliCommand for Command {
    async fn run(&self) -> Result<()> {
        log::warn!("Starting daemon...");

        let mut scheduler = JobScheduler::new();

        let timezone = Local::now().fixed_offset().timezone();
        log::debug!("Detected system timezone set to {timezone}");
        scheduler.set_timezone(timezone);

        let config = self.global_options.parse_config()?;
        let update_job = self.construct_update_job(config).await?;
        let update_job_uuid = scheduler.add(update_job);

        let shutdown = tokio_shutdown::Shutdown::new()?;
        loop {
            tokio::select! {
                () = shutdown.handle() => {
                    log::warn!("Got shutdown signal. Exiting...");
                    scheduler.remove(update_job_uuid);
                    break;
                },
                () = tokio::time::sleep(Duration::from_millis(500)) => {
                    scheduler.tick();
                }
            }
        }

        Ok(())
    }
}
