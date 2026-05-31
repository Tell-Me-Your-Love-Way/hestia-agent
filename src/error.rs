use salvo::prelude::*;

pub enum AgentError {
    Internal,
    Database,
    Validation,
    Encode,
    Decode,
}
#[async_trait]
impl Writer for AgentError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        match self {
            AgentError::Internal => {
                res.render(StatusCode::INTERNAL_SERVER_ERROR);
            },
            AgentError::Encode => {
                res.render(StatusCode::INTERNAL_SERVER_ERROR);
            },
            _ => {
                res.render(StatusCode::NOT_FOUND);
            }
        }
    }
}