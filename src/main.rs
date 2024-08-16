use crate::upload::UploadManager;

mod upload;

#[tokio::main]
async fn main() {
    const DOWNLOAD_URL: &str = "https://some-download-url.com/file/123";
    const BUCKET_NAME: &str = "example-streaming-upload";
    const FILE_NAME: &str = "demo.mp4";

    let upload_manager = UploadManager::new(BUCKET_NAME).await;

    // Download file to s3 bucket
    upload_manager
        .stream_mp4_to_s3(FILE_NAME, DOWNLOAD_URL)
        .await;
}
