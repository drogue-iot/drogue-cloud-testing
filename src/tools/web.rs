use crate::context::TestContext;

pub async fn with_screenshot<T, E>(
    result: Result<T, E>,
    name: &str,
    ctx: &mut TestContext,
) -> Result<T, E> {
    match result {
        Ok(result) => Ok(result),
        Err(err) => {
            ctx.web()
                .await
                .expect("Web Driver")
                .screenshot(format!("{}.png", name))
                .await
                .ok();
            Err(err)
        }
    }
}
