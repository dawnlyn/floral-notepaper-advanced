use super::types::SyncError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use reqwest::header::{CONTENT_TYPE, DATE};
use sha1::Sha1;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct OssConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
}

#[derive(Debug, Clone)]
pub struct ObjectInfo {
    pub key: String,
    pub last_modified: String,
    pub etag: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub content_length: u64,
    pub etag: String,
    pub last_modified: String,
}

pub struct OssClient {
    config: OssConfig,
    http_client: reqwest::Client,
}

impl OssClient {
    pub fn new(config: OssConfig) -> Result<Self, SyncError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            config,
            http_client,
        })
    }

    fn object_url(&self, key: &str) -> String {
        format!(
            "https://{}.{}/{}",
            self.config.bucket, self.config.endpoint, key
        )
    }

    fn canonicalized_resource(&self, key: &str) -> String {
        format!("/{}/{}", self.config.bucket, key)
    }

    fn sign_request(&self, method: &str, resource: &str, content_type: &str, date: &str) -> String {
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}\n{}",
            method,
            "",
            content_type,
            date,
            self.canonicalized_resource(resource)
        );

        let mut mac = Hmac::<Sha1>::new_from_slice(self.config.access_key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = BASE64.encode(mac.finalize().into_bytes());

        format!("OSS {}:{}", self.config.access_key_id, signature)
    }

    pub async fn put_object(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(), SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let authorization = self.sign_request("PUT", key, content_type, &date);

        let response = self
            .http_client
            .put(self.object_url(key))
            .header(DATE, &date)
            .header(CONTENT_TYPE, content_type)
            .header("Authorization", &authorization)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SyncError::new(
                "ossPut",
                format!("PUT {} failed: {} {}", key, status, text),
            ));
        }

        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let authorization = self.sign_request("GET", key, "", &date);

        let response = self
            .http_client
            .get(self.object_url(key))
            .header(DATE, &date)
            .header("Authorization", &authorization)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SyncError::new(
                "ossGet",
                format!("GET {} failed: {} {}", key, status, text),
            ));
        }

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn head_object(&self, key: &str) -> Result<ObjectMeta, SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let authorization = self.sign_request("HEAD", key, "", &date);

        let response = self
            .http_client
            .head(self.object_url(key))
            .header(DATE, &date)
            .header("Authorization", &authorization)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(SyncError::new(
                "ossHead",
                format!("HEAD {} failed: {}", key, status),
            ));
        }

        let headers = response.headers();
        let content_length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let etag = headers
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let last_modified = headers
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Ok(ObjectMeta {
            content_length,
            etag,
            last_modified,
        })
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let authorization = self.sign_request("DELETE", key, "", &date);

        let response = self
            .http_client
            .delete(self.object_url(key))
            .header(DATE, &date)
            .header("Authorization", &authorization)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SyncError::new(
                "ossDelete",
                format!("DELETE {} failed: {} {}", key, status, text),
            ));
        }

        Ok(())
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectInfo>, SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let encoded_prefix = percent_encode(prefix.as_bytes(), NON_ALPHANUMERIC).to_string();
        let resource = format!("?prefix={}", encoded_prefix);
        let authorization = self.sign_request("GET", &resource, "", &date);

        let url = format!(
            "https://{}.{}/?prefix={}",
            self.config.bucket, self.config.endpoint, encoded_prefix
        );

        let response = self
            .http_client
            .get(&url)
            .header(DATE, &date)
            .header("Authorization", &authorization)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SyncError::new(
                "ossList",
                format!("LIST prefix={} failed: {} {}", prefix, status, text),
            ));
        }

        let body = response.text().await?;
        self.parse_list_objects_response(&body)
    }

    fn parse_list_objects_response(&self, xml: &str) -> Result<Vec<ObjectInfo>, SyncError> {
        let mut objects = Vec::new();

        // Simple XML parsing for ListObjects response
        let contents_start = "<Contents>";
        let contents_end = "</Contents>";
        let mut pos = 0;

        while let Some(start) = xml[pos..].find(contents_start) {
            let start = pos + start;
            if let Some(end) = xml[start..].find(contents_end) {
                let content = &xml[start..start + end + contents_end.len()];

                let key = Self::extract_xml_value(content, "Key").unwrap_or_default();
                let last_modified =
                    Self::extract_xml_value(content, "LastModified").unwrap_or_default();
                let etag = Self::extract_xml_value(content, "ETag").unwrap_or_default();
                let size = Self::extract_xml_value(content, "Size")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0u64);

                objects.push(ObjectInfo {
                    key,
                    last_modified,
                    etag,
                    size,
                });

                pos = start + end + contents_end.len();
            } else {
                break;
            }
        }

        Ok(objects)
    }

    fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start = xml.find(&start_tag)? + start_tag.len();
        let end = xml[start..].find(&end_tag)?;
        Some(xml[start..start + end].to_string())
    }

    pub async fn test_connection(&self) -> Result<(), SyncError> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let authorization = self.sign_request("HEAD", "", "", &date);

        let url = format!("https://{}.{}/", self.config.bucket, self.config.endpoint);

        let response = self
            .http_client
            .head(&url)
            .header(DATE, &date)
            .header("Authorization", &authorization)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();

            return Err(SyncError::new(
                "ossConnection",
                format!("Connection test failed: {}", status),
            ));
        }

        Ok(())
    }
}
