use super::{HttpResponse, HttpTransport, ProviderRequest, TransportError};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

#[derive(Debug)]
pub struct ReqwestHttpTransport {
    client: Client,
}

#[derive(Debug)]
pub struct ReqwestTransportBuildError(reqwest::Error);

impl ReqwestHttpTransport {
    pub fn new(timeout: Duration) -> Result<Self, ReqwestTransportBuildError> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ReqwestTransportBuildError)?;
        Ok(Self { client })
    }
}

impl HttpTransport for ReqwestHttpTransport {
    fn send(&mut self, request: ProviderRequest) -> Result<HttpResponse, TransportError> {
        let response = self
            .client
            .post(request.url)
            .headers(headers(request.headers)?)
            .json(&request.body)
            .send()
            .map_err(map_reqwest_error)?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.to_string(), value.to_string()))
            })
            .collect();
        let body = response.text().map_err(map_reqwest_error)?;

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn headers(headers: Vec<(String, String)>) -> Result<HeaderMap, TransportError> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| TransportError::Network)?;
        let value = HeaderValue::from_str(&value).map_err(|_| TransportError::Network)?;
        map.insert(name, value);
    }
    Ok(map)
}

fn map_reqwest_error(error: reqwest::Error) -> TransportError {
    if error.is_timeout() {
        TransportError::Timeout
    } else {
        TransportError::Network
    }
}

impl std::fmt::Display for ReqwestTransportBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "failed to build reqwest transport: {}", self.0)
    }
}

impl std::error::Error for ReqwestTransportBuildError {}
