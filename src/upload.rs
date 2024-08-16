use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use futures_util::StreamExt;
use reqwest::{Client, IntoUrl};
use tracing::info;

pub struct UploadManager {
    client: Client,
    s3_client: S3Client,
    bucket_name: String,
}

impl UploadManager {
    pub async fn new(bucket_name: &str) -> Self {
        let client = Client::new();
        let aws_cfg = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let s3_client = S3Client::new(&aws_cfg);
        Self {
            client,
            s3_client,
            bucket_name: String::from(bucket_name),
        }
    }

    pub async fn stream_mp4_to_s3<U: IntoUrl>(&self, key: &str, url: U) {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .expect("must download mp4 from recall")
            .error_for_status()
            .expect("response to download mp4 must be successful");

        // Start multipart upload
        let create_multipart_upload_result = self
            .s3_client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .expect("must create multipart upload successfully");

        let upload_id = create_multipart_upload_result
            .upload_id()
            .expect("must receive a valid upload id");

        // Initialize the stream from the response
        let mut stream = response.bytes_stream();

        let mut parts = Vec::new();
        let mut buffer = Vec::new();
        let mut part_number = 1;
        let mut total_size = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            buffer.extend_from_slice(&chunk);
            total_size += chunk.len();

            // Upload when buffer reaches a certain size (e.g., 5 MB)
            if total_size >= 5 * 1024 * 1024 {
                let part = self
                    .s3_client
                    .upload_part()
                    .bucket(&self.bucket_name)
                    .key(key)
                    .upload_id(upload_id)
                    .part_number(part_number)
                    .body(buffer.clone().into())
                    .send()
                    .await
                    .unwrap();

                parts.push(
                    CompletedPart::builder()
                        .e_tag(part.e_tag.unwrap())
                        .part_number(part_number)
                        .build(),
                );

                buffer.clear();
                total_size = 0;
                part_number += 1;
            }
        }

        // Upload any remaining data as the last part
        if !buffer.is_empty() {
            let part = self
                .s3_client
                .upload_part()
                .bucket(&self.bucket_name)
                .key(key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(buffer.into())
                .send()
                .await
                .expect("can upload part successfully");

            parts.push(
                CompletedPart::builder()
                    .e_tag(part.e_tag.unwrap())
                    .part_number(part_number)
                    .build(),
            );
        }

        // Complete multipart upload
        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        self.s3_client
            .complete_multipart_upload()
            .bucket(&self.bucket_name)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .expect("multipart upload completes successfully");

        info!("File streamed and uploaded to S3 successfully!");
    }
}
