use crate::tests::{ValidTest, ValidTestParameter};
use crate::{Arguments, HasValidate, HasValidateArgs, Valid, ValidEx, VALIDATION_ERROR_STATUS};
use axum::extract::{FromRef, Path, Query};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use hyper::Method;
use once_cell::sync::Lazy;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::net::SocketAddr;
use std::ops::{Deref, RangeInclusive};
use std::sync::Arc;
use validator::{Validate, ValidateArgs, ValidationError};

#[derive(Clone, Deserialize, Serialize, Validate, Eq, PartialEq)]
#[cfg_attr(feature = "extra_protobuf", derive(prost::Message))]
#[cfg_attr(
    feature = "typed_multipart",
    derive(axum_typed_multipart::TryFromMultipart)
)]
pub struct Parameters {
    #[validate(range(min = 5, max = 10))]
    #[cfg_attr(feature = "extra_protobuf", prost(int32, tag = "1"))]
    v0: i32,
    #[validate(length(min = 1, max = 10))]
    #[cfg_attr(feature = "extra_protobuf", prost(string, tag = "2"))]
    v1: String,
}

#[derive(Clone, Deserialize, Serialize, Validate, Eq, PartialEq)]
#[cfg_attr(feature = "extra_protobuf", derive(prost::Message))]
#[cfg_attr(
    feature = "typed_multipart",
    derive(axum_typed_multipart::TryFromMultipart)
)]
pub struct ParametersEx {
    #[validate(custom(function = "validate_v0", arg = "&'v_a RangeInclusive<i32>"))]
    #[cfg_attr(feature = "extra_protobuf", prost(int32, tag = "1"))]
    v0: i32,
    #[validate(custom(function = "validate_v1", arg = "&'v_a RangeInclusive<usize>"))]
    #[cfg_attr(feature = "extra_protobuf", prost(string, tag = "2"))]
    v1: String,
}

fn validate_v0(v: i32, args: &RangeInclusive<i32>) -> Result<(), ValidationError> {
    args.contains(&v)
        .then_some(())
        .ok_or_else(|| ValidationError::new("v0 is out of range"))
}

fn validate_v1(v: &str, args: &RangeInclusive<usize>) -> Result<(), ValidationError> {
    args.contains(&v.len())
        .then_some(())
        .ok_or_else(|| ValidationError::new("v1 is invalid"))
}

#[derive(Debug, Clone)]
struct ParametersExValidationArgumentsInner {
    v0_range: RangeInclusive<i32>,
    v1_length_range: RangeInclusive<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ParametersExValidationArguments {
    inner: Arc<ParametersExValidationArgumentsInner>,
}

impl<'a> Arguments<'a> for ParametersExValidationArguments {
    type T = ParametersEx;
    fn get(&'a self) -> <ParametersEx as ValidateArgs<'a>>::Args {
        (&self.inner.v0_range, &self.inner.v1_length_range)
    }
}

impl Default for ParametersExValidationArgumentsInner {
    fn default() -> Self {
        Self {
            v0_range: 5..=10,
            v1_length_range: 1..=10,
        }
    }
}

static VALID_PARAMETERS: Lazy<Parameters> = Lazy::new(|| Parameters {
    v0: 5,
    v1: String::from("0123456789"),
});

static INVALID_PARAMETERS: Lazy<Parameters> = Lazy::new(|| Parameters {
    v0: 6,
    v1: String::from("01234567890"),
});

impl ValidTestParameter for Parameters {
    fn valid() -> &'static Self {
        VALID_PARAMETERS.deref()
    }

    fn error() -> &'static [(&'static str, &'static str)] {
        &[("not_v0_or_v1", "value")]
    }

    fn invalid() -> &'static Self {
        INVALID_PARAMETERS.deref()
    }
}

impl HasValidate for Parameters {
    type Validate = Parameters;

    fn get_validate(&self) -> &Self::Validate {
        self
    }
}

impl<'v> HasValidateArgs<'v> for ParametersEx {
    type ValidateArgs = ParametersEx;

    fn get_validate_args(&self) -> &Self::ValidateArgs {
        self
    }
}

#[derive(Debug, Clone, FromRef)]
struct MyState {
    param_validation_ctx: ParametersExValidationArguments,
    #[cfg(feature = "extra_typed_path")]
    typed_path_validation_ctx: extra_typed_path::TypedPathParamExValidationArguments,
}

#[tokio::test]
async fn test_main() -> anyhow::Result<()> {
    let state = MyState {
        param_validation_ctx: ParametersExValidationArguments::default(),
        #[cfg(feature = "extra_typed_path")]
        typed_path_validation_ctx: extra_typed_path::TypedPathParamExValidationArguments::default(),
    };

    let router = Router::new()
        .route(route::PATH, get(extract_path))
        .route(route::QUERY, get(extract_query))
        .route(route::FORM, post(extract_form))
        .route(route::JSON, post(extract_json))
        .route(route::PATH_EX, get(extract_path_ex))
        .route(route::QUERY_EX, get(extract_query_ex))
        .route(route::FORM_EX, post(extract_form_ex))
        .route(route::JSON_EX, post(extract_json_ex));

    #[cfg(feature = "typed_header")]
    let router = router
        .route(
            typed_header::route::TYPED_HEADER,
            post(typed_header::extract_typed_header),
        )
        .route(
            typed_header::route::TYPED_HEADER_EX,
            post(typed_header::extract_typed_header_ex),
        );

    #[cfg(feature = "typed_multipart")]
    let router = router
        .route(
            typed_multipart::route::TYPED_MULTIPART,
            post(typed_multipart::extract_typed_multipart),
        )
        .route(
            typed_multipart::route::TYPED_MULTIPART_EX,
            post(typed_multipart::extract_typed_multipart_ex),
        )
        .route(
            typed_multipart::route::BASE_MULTIPART,
            post(typed_multipart::extract_base_multipart),
        )
        .route(
            typed_multipart::route::BASE_MULTIPART_EX,
            post(typed_multipart::extract_base_multipart_ex),
        );

    #[cfg(feature = "extra")]
    let router = router
        .route(extra::route::CACHED, post(extra::extract_cached))
        .route(extra::route::CACHED_EX, post(extra::extract_cached_ex))
        .route(
            extra::route::WITH_REJECTION,
            post(extra::extract_with_rejection),
        )
        .route(
            extra::route::WITH_REJECTION_EX,
            post(extra::extract_with_rejection_ex),
        )
        .route(
            extra::route::WITH_REJECTION_VALID,
            post(extra::extract_with_rejection_valid),
        )
        .route(
            extra::route::WITH_REJECTION_VALID_EX,
            post(extra::extract_with_rejection_valid_ex),
        );

    #[cfg(feature = "extra_typed_path")]
    let router = router
        .route(
            extra_typed_path::route::EXTRA_TYPED_PATH,
            get(extra_typed_path::extract_extra_typed_path),
        )
        .route(
            extra_typed_path::route::EXTRA_TYPED_PATH_EX,
            get(extra_typed_path::extract_extra_typed_path_ex),
        );

    #[cfg(feature = "extra_query")]
    let router = router
        .route(
            extra_query::route::EXTRA_QUERY,
            post(extra_query::extract_extra_query),
        )
        .route(
            extra_query::route::EXTRA_QUERY_EX,
            post(extra_query::extract_extra_query_ex),
        );

    #[cfg(feature = "extra_form")]
    let router = router
        .route(
            extra_form::route::EXTRA_FORM,
            post(extra_form::extract_extra_form),
        )
        .route(
            extra_form::route::EXTRA_FORM_EX,
            post(extra_form::extract_extra_form_ex),
        );

    #[cfg(feature = "extra_protobuf")]
    let router = router
        .route(
            extra_protobuf::route::EXTRA_PROTOBUF,
            post(extra_protobuf::extract_extra_protobuf),
        )
        .route(
            extra_protobuf::route::EXTRA_PROTOBUF_EX,
            post(extra_protobuf::extract_extra_protobuf_ex),
        );

    #[cfg(feature = "yaml")]
    let router = router
        .route(yaml::route::YAML, post(yaml::extract_yaml))
        .route(yaml::route::YAML_EX, post(yaml::extract_yaml_ex));

    #[cfg(feature = "msgpack")]
    let router = router
        .route(msgpack::route::MSGPACK, post(msgpack::extract_msgpack))
        .route(
            msgpack::route::MSGPACK_EX,
            post(msgpack::extract_msgpack_ex),
        )
        .route(
            msgpack::route::MSGPACK_RAW,
            post(msgpack::extract_msgpack_raw),
        )
        .route(
            msgpack::route::MSGPACK_RAW_EX,
            post(msgpack::extract_msgpack_raw_ex),
        );

    let router = router.with_state(state);

    let server = axum::Server::bind(&SocketAddr::from(([0u8, 0, 0, 0], 0u16)))
        .serve(router.into_make_service());
    let server_addr = server.local_addr();
    println!("Axum server address: {}.", server_addr);

    let (server_guard, close) = tokio::sync::oneshot::channel::<()>();
    let server_handle = tokio::spawn(server.with_graceful_shutdown(async move {
        let _ = close.await;
    }));

    let server_url = format!("http://{}", server_addr);
    let test_executor = TestExecutor::from(Url::parse(&format!("http://{}", server_addr))?);

    async fn test_extra_path(
        test_executor: &TestExecutor,
        route: &str,
        server_url: &str,
    ) -> anyhow::Result<()> {
        let path_type_name = type_name::<Path<Parameters>>();
        let valid_path_response = test_executor
            .client()
            .get(format!(
                "{}/{route}/{}/{}",
                server_url, VALID_PARAMETERS.v0, VALID_PARAMETERS.v1
            ))
            .send()
            .await?;
        assert_eq!(
            valid_path_response.status(),
            StatusCode::OK,
            "Valid '{}' test failed.",
            path_type_name
        );

        let error_path_response = test_executor
            .client()
            .get(format!("{}/{route}/not_i32/path", server_url))
            .send()
            .await?;
        assert_eq!(
            error_path_response.status(),
            StatusCode::BAD_REQUEST,
            "Error '{}' test failed.",
            path_type_name
        );

        let invalid_path_response = test_executor
            .client()
            .get(format!(
                "{}/{route}/{}/{}",
                server_url, INVALID_PARAMETERS.v0, INVALID_PARAMETERS.v1
            ))
            .send()
            .await?;
        assert_eq!(
            invalid_path_response.status(),
            VALIDATION_ERROR_STATUS,
            "Invalid '{}' test failed.",
            path_type_name
        );
        #[cfg(feature = "into_json")]
        check_json(path_type_name, invalid_path_response).await;
        println!("All {} tests passed.", path_type_name);
        Ok(())
    }

    test_extra_path(&test_executor, "path", &server_url).await?;
    test_extra_path(&test_executor, "path_ex", &server_url).await?;

    // Valid
    test_executor
        .execute::<Query<Parameters>>(Method::GET, route::QUERY)
        .await?;

    // ValidEx
    test_executor
        .execute::<Query<Parameters>>(Method::GET, route::QUERY_EX)
        .await?;

    // Valid
    test_executor
        .execute::<Form<Parameters>>(Method::POST, route::FORM)
        .await?;

    // ValidEx
    test_executor
        .execute::<Form<Parameters>>(Method::POST, route::FORM_EX)
        .await?;

    // Valid
    test_executor
        .execute::<Json<Parameters>>(Method::POST, route::JSON)
        .await?;

    // ValidEx
    test_executor
        .execute::<Json<Parameters>>(Method::POST, route::JSON_EX)
        .await?;

    #[cfg(feature = "typed_header")]
    {
        use axum::TypedHeader;
        // Valid
        test_executor
            .execute::<TypedHeader<Parameters>>(Method::POST, typed_header::route::TYPED_HEADER)
            .await?;

        // ValidEx
        test_executor
            .execute::<TypedHeader<Parameters>>(Method::POST, typed_header::route::TYPED_HEADER_EX)
            .await?;
    }

    #[cfg(feature = "typed_multipart")]
    {
        use axum_typed_multipart::{BaseMultipart, TypedMultipart, TypedMultipartError};

        // Valid
        test_executor
            .execute::<BaseMultipart<Parameters, TypedMultipartError>>(
                Method::POST,
                typed_multipart::route::BASE_MULTIPART,
            )
            .await?;

        // ValidEx
        test_executor
            .execute::<BaseMultipart<Parameters, TypedMultipartError>>(
                Method::POST,
                typed_multipart::route::BASE_MULTIPART_EX,
            )
            .await?;

        // Valid
        test_executor
            .execute::<TypedMultipart<Parameters>>(
                Method::POST,
                typed_multipart::route::TYPED_MULTIPART,
            )
            .await?;

        // ValidEx
        test_executor
            .execute::<TypedMultipart<Parameters>>(
                Method::POST,
                typed_multipart::route::TYPED_MULTIPART_EX,
            )
            .await?;
    }

    #[cfg(feature = "extra")]
    {
        use axum_extra::extract::{Cached, WithRejection};
        use extra::{
            ParametersRejection, ValidWithRejectionRejection, WithRejectionValidRejection,
        };
        test_executor
            .execute::<Cached<Parameters>>(Method::POST, extra::route::CACHED)
            .await?;
        test_executor
            .execute::<Cached<Parameters>>(Method::POST, extra::route::CACHED_EX)
            .await?;
        test_executor
            .execute::<WithRejection<Parameters, ValidWithRejectionRejection>>(
                Method::POST,
                extra::route::WITH_REJECTION,
            )
            .await?;
        test_executor
            .execute::<WithRejection<Parameters, ValidWithRejectionRejection>>(
                Method::POST,
                extra::route::WITH_REJECTION_EX,
            )
            .await?;
        test_executor
            .execute::<WithRejection<Valid<Parameters>, WithRejectionValidRejection<ParametersRejection>>>(
                Method::POST,
                extra::route::WITH_REJECTION_VALID,
            )
            .await?;
        test_executor
            .execute::<WithRejection<Valid<Parameters>, WithRejectionValidRejection<ParametersRejection>>>(
                Method::POST,
                extra::route::WITH_REJECTION_VALID_EX,
            )
            .await?;
    }

    #[cfg(feature = "extra_typed_path")]
    {
        async fn test_extra_typed_path(
            test_executor: &TestExecutor,
            route: &str,
            server_url: &str,
        ) -> anyhow::Result<()> {
            let extra_typed_path_type_name = "T: TypedPath";
            let valid_extra_typed_path_response = test_executor
                .client()
                .get(format!(
                    "{}/{route}/{}/{}",
                    server_url, VALID_PARAMETERS.v0, VALID_PARAMETERS.v1
                ))
                .send()
                .await?;
            assert_eq!(
                valid_extra_typed_path_response.status(),
                StatusCode::OK,
                "Valid '{}' test failed.",
                extra_typed_path_type_name
            );

            let error_extra_typed_path_response = test_executor
                .client()
                .get(format!("{}/{route}/not_i32/path", server_url))
                .send()
                .await?;
            assert_eq!(
                error_extra_typed_path_response.status(),
                StatusCode::BAD_REQUEST,
                "Error '{}' test failed.",
                extra_typed_path_type_name
            );

            let invalid_extra_typed_path_response = test_executor
                .client()
                .get(format!(
                    "{}/{route}/{}/{}",
                    server_url, INVALID_PARAMETERS.v0, INVALID_PARAMETERS.v1
                ))
                .send()
                .await?;
            assert_eq!(
                invalid_extra_typed_path_response.status(),
                VALIDATION_ERROR_STATUS,
                "Invalid '{}' test failed.",
                extra_typed_path_type_name
            );
            #[cfg(feature = "into_json")]
            check_json(
                extra_typed_path_type_name,
                invalid_extra_typed_path_response,
            )
            .await;
            println!("All {} tests passed.", extra_typed_path_type_name);
            Ok(())
        }

        test_extra_typed_path(&test_executor, "extra_typed_path", &server_url).await?;
        test_extra_typed_path(&test_executor, "extra_typed_path_ex", &server_url).await?;
    }

    #[cfg(feature = "extra_query")]
    {
        use axum_extra::extract::Query;
        test_executor
            .execute::<Query<Parameters>>(Method::POST, extra_query::route::EXTRA_QUERY)
            .await?;
        test_executor
            .execute::<Query<Parameters>>(Method::POST, extra_query::route::EXTRA_QUERY_EX)
            .await?;
    }

    #[cfg(feature = "extra_form")]
    {
        use axum_extra::extract::Form;
        test_executor
            .execute::<Form<Parameters>>(Method::POST, extra_form::route::EXTRA_FORM)
            .await?;
        test_executor
            .execute::<Form<Parameters>>(Method::POST, extra_form::route::EXTRA_FORM_EX)
            .await?;
    }

    #[cfg(feature = "extra_protobuf")]
    {
        use axum_extra::protobuf::Protobuf;
        test_executor
            .execute::<Protobuf<Parameters>>(Method::POST, extra_protobuf::route::EXTRA_PROTOBUF)
            .await?;
        test_executor
            .execute::<Protobuf<Parameters>>(Method::POST, extra_protobuf::route::EXTRA_PROTOBUF_EX)
            .await?;
    }

    #[cfg(feature = "yaml")]
    {
        use axum_yaml::Yaml;
        test_executor
            .execute::<Yaml<Parameters>>(Method::POST, yaml::route::YAML)
            .await?;
        test_executor
            .execute::<Yaml<Parameters>>(Method::POST, yaml::route::YAML_EX)
            .await?;
    }

    #[cfg(feature = "msgpack")]
    {
        use axum_msgpack::{MsgPack, MsgPackRaw};
        test_executor
            .execute::<MsgPack<Parameters>>(Method::POST, msgpack::route::MSGPACK)
            .await?;
        test_executor
            .execute::<MsgPack<Parameters>>(Method::POST, msgpack::route::MSGPACK_EX)
            .await?;
        test_executor
            .execute::<MsgPackRaw<Parameters>>(Method::POST, msgpack::route::MSGPACK_RAW)
            .await?;
        test_executor
            .execute::<MsgPackRaw<Parameters>>(Method::POST, msgpack::route::MSGPACK_RAW_EX)
            .await?;
    }

    drop(server_guard);
    server_handle.await??;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TestExecutor {
    client: reqwest::Client,
    server_url: Url,
}

impl From<Url> for TestExecutor {
    fn from(server_url: Url) -> Self {
        Self {
            client: Default::default(),
            server_url,
        }
    }
}

impl TestExecutor {
    /// Execute all tests
    pub async fn execute<T: ValidTest>(&self, method: Method, route: &str) -> anyhow::Result<()> {
        let url = {
            let mut url_builder = self.server_url.clone();
            url_builder.set_path(route);
            url_builder
        };

        let type_name = type_name::<T>();

        let valid_builder = self.client.request(method.clone(), url.clone());
        let valid_response = T::set_valid_request(valid_builder).send().await?;
        assert_eq!(
            valid_response.status(),
            StatusCode::OK,
            "Valid '{}' test failed.",
            type_name
        );

        let error_builder = self.client.request(method.clone(), url.clone());
        let error_response = T::set_error_request(error_builder).send().await?;
        assert_eq!(
            error_response.status(),
            T::ERROR_STATUS_CODE,
            "Error '{}' test failed.",
            type_name
        );

        let invalid_builder = self.client.request(method, url);
        let invalid_response = T::set_invalid_request(invalid_builder).send().await?;
        assert_eq!(
            invalid_response.status(),
            T::INVALID_STATUS_CODE,
            "Invalid '{}' test failed.",
            type_name
        );
        #[cfg(feature = "into_json")]
        if T::JSON_SERIALIZABLE {
            check_json(type_name, invalid_response).await;
        }

        println!("All '{}' tests passed.", type_name);

        Ok(())
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

/// Check if the response is a json response
#[cfg(feature = "into_json")]
pub async fn check_json(type_name: &'static str, response: reqwest::Response) {
    assert_eq!(
        response.headers()[axum::http::header::CONTENT_TYPE],
        axum::http::HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        "'{}' rejection into json test failed",
        type_name
    );
    assert!(response.json::<serde_json::Value>().await.is_ok());
}

mod route {
    pub const PATH: &str = "/path/:v0/:v1";
    pub const PATH_EX: &str = "/path_ex/:v0/:v1";
    pub const QUERY: &str = "/query";
    pub const QUERY_EX: &str = "/query_ex";
    pub const FORM: &str = "/form";
    pub const FORM_EX: &str = "/form_ex";
    pub const JSON: &str = "/json";
    pub const JSON_EX: &str = "/json_ex";
}

async fn extract_path(Valid(Path(parameters)): Valid<Path<Parameters>>) -> StatusCode {
    validate_again(parameters)
}

async fn extract_path_ex(
    ValidEx(Path(parameters), args): ValidEx<Path<ParametersEx>, ParametersExValidationArguments>,
) -> StatusCode {
    validate_again_ex(parameters, args.get())
}

async fn extract_query(Valid(Query(parameters)): Valid<Query<Parameters>>) -> StatusCode {
    validate_again(parameters)
}

async fn extract_query_ex(
    ValidEx(Query(parameters), args): ValidEx<Query<ParametersEx>, ParametersExValidationArguments>,
) -> StatusCode {
    validate_again_ex(parameters, args.get())
}

async fn extract_form(Valid(Form(parameters)): Valid<Form<Parameters>>) -> StatusCode {
    validate_again(parameters)
}

async fn extract_form_ex(
    ValidEx(Form(parameters), args): ValidEx<Form<ParametersEx>, ParametersExValidationArguments>,
) -> StatusCode {
    validate_again_ex(parameters, args.get())
}

async fn extract_json(Valid(Json(parameters)): Valid<Json<Parameters>>) -> StatusCode {
    validate_again(parameters)
}

async fn extract_json_ex(
    ValidEx(Json(parameters), args): ValidEx<Json<ParametersEx>, ParametersExValidationArguments>,
) -> StatusCode {
    validate_again_ex(parameters, args.get())
}

fn validate_again<V: Validate>(validate: V) -> StatusCode {
    // The `Valid` extractor has validated the `parameters` once,
    // it should have returned `400 BAD REQUEST` if the `parameters` were invalid,
    // Let's validate them again to check if the `Valid` extractor works well.
    // If it works properly, this function will never return `500 INTERNAL SERVER ERROR`
    match validate.validate() {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn validate_again_ex<'v, V: ValidateArgs<'v>>(
    validate: V,
    args: <V as ValidateArgs<'v>>::Args,
) -> StatusCode {
    // The `Valid` extractor has validated the `parameters` once,
    // it should have returned `400 BAD REQUEST` if the `parameters` were invalid,
    // Let's validate them again to check if the `Valid` extractor works well.
    // If it works properly, this function will never return `500 INTERNAL SERVER ERROR`
    match validate.validate_args(args) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(feature = "typed_header")]
mod typed_header {

    pub(crate) mod route {
        pub const TYPED_HEADER: &str = "/typed_header";
        pub const TYPED_HEADER_EX: &str = "/typed_header_ex";
    }

    use super::{validate_again, Parameters};
    use crate::test::{validate_again_ex, ParametersEx, ParametersExValidationArguments};
    use crate::{Arguments, Valid, ValidEx};
    use axum::headers::{Error, Header, HeaderName, HeaderValue};
    use axum::http::StatusCode;
    use axum::TypedHeader;

    pub static AXUM_VALID_PARAMETERS: HeaderName = HeaderName::from_static("axum-valid-parameters");

    pub(super) async fn extract_typed_header(
        Valid(TypedHeader(parameters)): Valid<TypedHeader<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub(super) async fn extract_typed_header_ex(
        ValidEx(TypedHeader(parameters), args): ValidEx<
            TypedHeader<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }

    impl Header for Parameters {
        fn name() -> &'static HeaderName {
            &AXUM_VALID_PARAMETERS
        }

        fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
        where
            Self: Sized,
            I: Iterator<Item = &'i HeaderValue>,
        {
            let value = values.next().ok_or_else(Error::invalid)?;
            let src = std::str::from_utf8(value.as_bytes()).map_err(|_| Error::invalid())?;
            let split = src.split(',').collect::<Vec<_>>();
            match split.as_slice() {
                [v0, v1] => Ok(Parameters {
                    v0: v0.parse().map_err(|_| Error::invalid())?,
                    v1: v1.to_string(),
                }),
                _ => Err(Error::invalid()),
            }
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let v0 = self.v0.to_string();
            let mut vec = Vec::with_capacity(v0.len() + 1 + self.v1.len());
            vec.extend_from_slice(v0.as_bytes());
            vec.push(b',');
            vec.extend_from_slice(self.v1.as_bytes());
            let value = HeaderValue::from_bytes(&vec).expect("Failed to build header");
            values.extend(::std::iter::once(value));
        }
    }

    impl Header for ParametersEx {
        fn name() -> &'static HeaderName {
            &AXUM_VALID_PARAMETERS
        }

        fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
        where
            Self: Sized,
            I: Iterator<Item = &'i HeaderValue>,
        {
            let value = values.next().ok_or_else(Error::invalid)?;
            let src = std::str::from_utf8(value.as_bytes()).map_err(|_| Error::invalid())?;
            let split = src.split(',').collect::<Vec<_>>();
            match split.as_slice() {
                [v0, v1] => Ok(ParametersEx {
                    v0: v0.parse().map_err(|_| Error::invalid())?,
                    v1: v1.to_string(),
                }),
                _ => Err(Error::invalid()),
            }
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let v0 = self.v0.to_string();
            let mut vec = Vec::with_capacity(v0.len() + 1 + self.v1.len());
            vec.extend_from_slice(v0.as_bytes());
            vec.push(b',');
            vec.extend_from_slice(self.v1.as_bytes());
            let value = HeaderValue::from_bytes(&vec).expect("Failed to build header");
            values.extend(::std::iter::once(value));
        }
    }

    #[test]
    fn parameter_is_header() -> anyhow::Result<()> {
        let parameter = Parameters {
            v0: 123456,
            v1: "111111".to_string(),
        };
        let mut vec = Vec::new();
        parameter.encode(&mut vec);
        let mut iter = vec.iter();
        assert_eq!(parameter, Parameters::decode(&mut iter)?);
        Ok(())
    }
}

#[cfg(feature = "typed_multipart")]
mod typed_multipart {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_typed_multipart::{BaseMultipart, TypedMultipart, TypedMultipartError};

    pub mod route {
        pub const TYPED_MULTIPART: &str = "/typed_multipart";
        pub const TYPED_MULTIPART_EX: &str = "/typed_multipart_ex";
        pub const BASE_MULTIPART: &str = "/base_multipart";
        pub const BASE_MULTIPART_EX: &str = "/base_multipart_ex";
    }

    impl From<&Parameters> for reqwest::multipart::Form {
        fn from(value: &Parameters) -> Self {
            reqwest::multipart::Form::new()
                .text("v0", value.v0.to_string())
                .text("v1", value.v1.clone())
        }
    }

    pub(super) async fn extract_typed_multipart(
        Valid(TypedMultipart(parameters)): Valid<TypedMultipart<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub(super) async fn extract_typed_multipart_ex(
        ValidEx(TypedMultipart(parameters), args): ValidEx<
            TypedMultipart<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }

    pub(super) async fn extract_base_multipart(
        Valid(BaseMultipart { data, .. }): Valid<BaseMultipart<Parameters, TypedMultipartError>>,
    ) -> StatusCode {
        validate_again(data)
    }

    pub(super) async fn extract_base_multipart_ex(
        ValidEx(BaseMultipart { data, .. }, args): ValidEx<
            BaseMultipart<ParametersEx, TypedMultipartError>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(data, args.get())
    }
}

#[cfg(feature = "extra")]
mod extra {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::tests::{Rejection, ValidTest, ValidTestParameter};
    use crate::{Arguments, Valid, ValidEx, ValidRejection};
    use axum::extract::FromRequestParts;
    use axum::http::request::Parts;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum_extra::extract::{Cached, WithRejection};
    use reqwest::RequestBuilder;

    pub mod route {
        pub const CACHED: &str = "/cached";
        pub const CACHED_EX: &str = "/cached_ex";
        pub const WITH_REJECTION: &str = "/with_rejection";
        pub const WITH_REJECTION_EX: &str = "/with_rejection_ex";
        pub const WITH_REJECTION_VALID: &str = "/with_rejection_valid";
        pub const WITH_REJECTION_VALID_EX: &str = "/with_rejection_valid_ex";
    }
    pub const PARAMETERS_HEADER: &str = "parameters-header";
    pub const CACHED_REJECTION_STATUS: StatusCode = StatusCode::FORBIDDEN;

    //  1.2. Define you own `Rejection` type and implement `IntoResponse` for it.
    pub enum ParametersRejection {
        Null,
        InvalidJson(serde_json::error::Error),
    }

    impl IntoResponse for ParametersRejection {
        fn into_response(self) -> Response {
            match self {
                ParametersRejection::Null => {
                    (CACHED_REJECTION_STATUS, "My-Data header is missing").into_response()
                }
                ParametersRejection::InvalidJson(e) => (
                    CACHED_REJECTION_STATUS,
                    format!("My-Data is not valid json string: {e}"),
                )
                    .into_response(),
            }
        }
    }

    //  1.3. Implement your extractor (`FromRequestParts` or `FromRequest`)
    #[axum::async_trait]
    impl<S> FromRequestParts<S> for Parameters
    where
        S: Send + Sync,
    {
        type Rejection = ParametersRejection;

        async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
            let Some(value) = parts.headers.get(PARAMETERS_HEADER) else {
                return Err(ParametersRejection::Null);
            };

            serde_json::from_slice(value.as_bytes()).map_err(ParametersRejection::InvalidJson)
        }
    }

    #[axum::async_trait]
    impl<S> FromRequestParts<S> for ParametersEx
    where
        S: Send + Sync,
    {
        type Rejection = ParametersRejection;

        async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
            let Some(value) = parts.headers.get(PARAMETERS_HEADER) else {
                return Err(ParametersRejection::Null);
            };

            serde_json::from_slice(value.as_bytes()).map_err(ParametersRejection::InvalidJson)
        }
    }

    impl ValidTest for Parameters {
        const ERROR_STATUS_CODE: StatusCode = CACHED_REJECTION_STATUS;

        fn set_valid_request(builder: RequestBuilder) -> RequestBuilder {
            builder.header(
                PARAMETERS_HEADER,
                serde_json::to_string(Parameters::valid()).expect("Failed to serialize parameters"),
            )
        }

        fn set_error_request(builder: RequestBuilder) -> RequestBuilder {
            builder.header(
                PARAMETERS_HEADER,
                serde_json::to_string(Parameters::error()).expect("Failed to serialize parameters"),
            )
        }

        fn set_invalid_request(builder: RequestBuilder) -> RequestBuilder {
            builder.header(
                PARAMETERS_HEADER,
                serde_json::to_string(Parameters::invalid())
                    .expect("Failed to serialize parameters"),
            )
        }
    }

    pub struct ValidWithRejectionRejection {
        inner: ParametersRejection,
    }

    impl Rejection for ValidWithRejectionRejection {
        const STATUS_CODE: StatusCode = StatusCode::CONFLICT;
    }

    impl IntoResponse for ValidWithRejectionRejection {
        fn into_response(self) -> Response {
            let mut response = self.inner.into_response();
            *response.status_mut() = Self::STATUS_CODE;
            response
        }
    }

    // satisfy the `WithRejection`'s extractor trait bound
    // R: From<E::Rejection> + IntoResponse
    impl From<ParametersRejection> for ValidWithRejectionRejection {
        fn from(inner: ParametersRejection) -> Self {
            Self { inner }
        }
    }

    pub async fn extract_cached(
        Valid(Cached(parameters)): Valid<Cached<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_cached_ex(
        ValidEx(Cached(parameters), args): ValidEx<
            Cached<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }

    pub async fn extract_with_rejection(
        Valid(WithRejection(parameters, _)): Valid<
            WithRejection<Parameters, ValidWithRejectionRejection>,
        >,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_with_rejection_ex(
        ValidEx(WithRejection(parameters, _), args): ValidEx<
            WithRejection<ParametersEx, ValidWithRejectionRejection>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }

    pub struct WithRejectionValidRejection<E> {
        inner: ValidRejection<E>,
    }

    impl<E> From<ValidRejection<E>> for WithRejectionValidRejection<E> {
        fn from(inner: ValidRejection<E>) -> Self {
            Self { inner }
        }
    }

    impl<E: IntoResponse> IntoResponse for WithRejectionValidRejection<E> {
        fn into_response(self) -> Response {
            let mut res = self.inner.into_response();
            *res.status_mut() = StatusCode::IM_A_TEAPOT;
            res
        }
    }

    pub async fn extract_with_rejection_valid(
        WithRejection(Valid(parameters), _): WithRejection<
            Valid<Parameters>,
            WithRejectionValidRejection<ParametersRejection>,
        >,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_with_rejection_valid_ex(
        WithRejection(ValidEx(parameters, args), _): WithRejection<
            ValidEx<ParametersEx, ParametersExValidationArguments>,
            WithRejectionValidRejection<ParametersRejection>,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}

#[cfg(feature = "extra_typed_path")]
mod extra_typed_path {
    use crate::test::{validate_again, validate_again_ex};
    use crate::{Arguments, HasValidate, HasValidateArgs, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_extra::routing::TypedPath;
    use serde::Deserialize;
    use std::ops::RangeInclusive;
    use validator::{Validate, ValidateArgs};

    pub mod route {
        pub const EXTRA_TYPED_PATH: &str = "/extra_typed_path/:v0/:v1";
        pub const EXTRA_TYPED_PATH_EX: &str = "/extra_typed_path_ex/:v0/:v1";
    }

    #[derive(Validate, TypedPath, Deserialize)]
    #[typed_path("/extra_typed_path/:v0/:v1")]
    pub struct TypedPathParam {
        #[validate(range(min = 5, max = 10))]
        v0: i32,
        #[validate(length(min = 1, max = 10))]
        v1: String,
    }

    impl HasValidate for TypedPathParam {
        type Validate = Self;

        fn get_validate(&self) -> &Self::Validate {
            self
        }
    }

    pub async fn extract_extra_typed_path(Valid(param): Valid<TypedPathParam>) -> StatusCode {
        validate_again(param)
    }

    #[derive(Validate, TypedPath, Deserialize)]
    #[typed_path("/extra_typed_path_ex/:v0/:v1")]
    pub struct TypedPathParamEx {
        #[validate(custom(function = "super::validate_v0", arg = "&'v_a RangeInclusive<i32>"))]
        v0: i32,
        #[validate(custom(function = "super::validate_v1", arg = "&'v_a RangeInclusive<usize>"))]
        v1: String,
    }

    impl<'v> HasValidateArgs<'v> for TypedPathParamEx {
        type ValidateArgs = Self;

        fn get_validate_args(&self) -> &Self::ValidateArgs {
            self
        }
    }

    #[derive(Debug, Clone)]
    pub struct TypedPathParamExValidationArguments {
        v0_range: RangeInclusive<i32>,
        v1_length_range: RangeInclusive<usize>,
    }

    impl Default for TypedPathParamExValidationArguments {
        fn default() -> Self {
            Self {
                v0_range: 5..=10,
                v1_length_range: 1..=10,
            }
        }
    }

    impl<'a> Arguments<'a> for TypedPathParamExValidationArguments {
        type T = TypedPathParamEx;
        fn get(&'a self) -> <TypedPathParamEx as ValidateArgs<'a>>::Args {
            (&self.v0_range, &self.v1_length_range)
        }
    }

    pub async fn extract_extra_typed_path_ex(
        ValidEx(param, args): ValidEx<TypedPathParamEx, TypedPathParamExValidationArguments>,
    ) -> StatusCode {
        validate_again_ex(param, args.get())
    }
}

#[cfg(feature = "extra_query")]
mod extra_query {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_extra::extract::Query;

    pub mod route {
        pub const EXTRA_QUERY: &str = "/extra_query";
        pub const EXTRA_QUERY_EX: &str = "/extra_query_ex";
    }

    pub async fn extract_extra_query(
        Valid(Query(parameters)): Valid<Query<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_extra_query_ex(
        ValidEx(Query(parameters), args): ValidEx<
            Query<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}

#[cfg(feature = "extra_form")]
mod extra_form {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_extra::extract::Form;

    pub mod route {
        pub const EXTRA_FORM: &str = "/extra_form";
        pub const EXTRA_FORM_EX: &str = "/extra_form_ex";
    }

    pub async fn extract_extra_form(
        Valid(Form(parameters)): Valid<Form<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_extra_form_ex(
        ValidEx(Form(parameters), args): ValidEx<
            Form<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}

#[cfg(feature = "extra_protobuf")]
mod extra_protobuf {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_extra::protobuf::Protobuf;

    pub mod route {
        pub const EXTRA_PROTOBUF: &str = "/extra_protobuf";
        pub const EXTRA_PROTOBUF_EX: &str = "/extra_protobuf_ex";
    }

    pub async fn extract_extra_protobuf(
        Valid(Protobuf(parameters)): Valid<Protobuf<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_extra_protobuf_ex(
        ValidEx(Protobuf(parameters), args): ValidEx<
            Protobuf<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}

#[cfg(feature = "yaml")]
mod yaml {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_yaml::Yaml;

    pub mod route {
        pub const YAML: &str = "/yaml";
        pub const YAML_EX: &str = "/yaml_ex";
    }

    pub async fn extract_yaml(Valid(Yaml(parameters)): Valid<Yaml<Parameters>>) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_yaml_ex(
        ValidEx(Yaml(parameters), args): ValidEx<
            Yaml<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}

#[cfg(feature = "msgpack")]
mod msgpack {
    use crate::test::{
        validate_again, validate_again_ex, Parameters, ParametersEx,
        ParametersExValidationArguments,
    };
    use crate::{Arguments, Valid, ValidEx};
    use axum::http::StatusCode;
    use axum_msgpack::{MsgPack, MsgPackRaw};

    pub mod route {
        pub const MSGPACK: &str = "/msgpack";
        pub const MSGPACK_EX: &str = "/msgpack_ex";
        pub const MSGPACK_RAW: &str = "/msgpack_raw";
        pub const MSGPACK_RAW_EX: &str = "/msgpack_raw_ex";
    }

    pub async fn extract_msgpack(
        Valid(MsgPack(parameters)): Valid<MsgPack<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_msgpack_ex(
        ValidEx(MsgPack(parameters), args): ValidEx<
            MsgPack<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }

    pub async fn extract_msgpack_raw(
        Valid(MsgPackRaw(parameters)): Valid<MsgPackRaw<Parameters>>,
    ) -> StatusCode {
        validate_again(parameters)
    }

    pub async fn extract_msgpack_raw_ex(
        ValidEx(MsgPackRaw(parameters), args): ValidEx<
            MsgPackRaw<ParametersEx>,
            ParametersExValidationArguments,
        >,
    ) -> StatusCode {
        validate_again_ex(parameters, args.get())
    }
}
