use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use redis::{ Commands };
use tonic::Code;
use tonic::{transport::Server, Request, Response, Status};

use leaderboard::leaderboard_server::{Leaderboard, LeaderboardServer};
use leaderboard::{
    ListLeaderboardScoresInput,
    ListLeaderboardScoresReply,
    ListLeaderboardsInput,
    ListLeaderboardsReply,
    UpdateScoreInput,
    UpdateScoreReply,
    ScoreObj
};

pub mod leaderboard {
    tonic::include_proto!("leaderboard");
}

pub struct Service {
    rd_client: redis::Client
}

impl Service {
    pub fn new(rd_client : redis::Client) -> Self {
        Self {
            rd_client
        }
    }
}

#[tonic::async_trait]
impl Leaderboard for Service {
    async fn list_leaderboard_scores(
        &self,
        request: Request<ListLeaderboardScoresInput>,
    ) -> Result<Response<ListLeaderboardScoresReply>, Status> {
        let conn = self.rd_client.get_connection();
        if conn.is_err() {
           return Err(Status::new(Code::Internal, conn.err().unwrap().to_string()));
        }

        let mut c = conn.unwrap();

        let min: isize = i64::MIN.try_into().unwrap();
        let max: isize = i64::MAX.try_into().unwrap();

        let id = request.into_inner().id;
        let key: String = format!("leaderboards:{}", id);
        let cmd_result: Vec<String> = c
        .zrevrange_withscores(key,min, max)
        .unwrap();

        let all_scoreobjs: Vec<ScoreObj> = cmd_result.chunks(2)
        .into_iter()
        .map(|r|
            ScoreObj{
                username: r[0].to_string(),
                score: r[1].parse::<i64>().unwrap()
            }).collect();

        let reply = leaderboard::ListLeaderboardScoresReply {
            id: id.to_string(),
            scores: all_scoreobjs,
        };

        Ok(Response::new(reply))
    }

    async fn list_leaderboards(
        &self,
        _request: Request<ListLeaderboardsInput>,
    ) -> Result<Response<ListLeaderboardsReply>, Status> {
        let conn = self.rd_client.get_connection();
        if conn.is_err() {
           return Err(Status::new(Code::Internal, conn.err().unwrap().to_string()));
        }

        let mut c = conn.unwrap();

        let l: Vec<String> = c.keys("leaderboards:*").unwrap();

        let ids: Vec<String> = l.into_iter().map(|i| i.replace("leaderboards:", "")).collect();

        let reply = leaderboard::ListLeaderboardsReply {
            result: ids,
        };

        Ok(Response::new(reply))
    }

    async fn update_score(
        &self,
        request: Request<UpdateScoreInput>,
    ) -> Result<Response<UpdateScoreReply>, Status> {
        let conn = self.rd_client.get_connection();
        if conn.is_err() {
           return Err(Status::new(Code::Internal, conn.err().unwrap().to_string()));
        }

        let mut c = conn.unwrap();

        let r = request.into_inner();
        let id = r.id;
        let opt_score = r.score;
        if opt_score.is_none() {
            return Err(Status::new(Code::Internal, "missing"))
        }

        let arg_score = opt_score.unwrap();

        let key: String = format!("leaderboards:{}", id);
        let cmd_result: u32 = c.zadd(key, arg_score.username.to_string(), arg_score.score.to_string()).unwrap();

        if cmd_result != 1 {
           return Err(Status::new(Code::Internal, "failed to update scores"));
        }

        let reply = leaderboard::UpdateScoreReply {
            score: Some(ScoreObj{
                username: arg_score.username.to_string(),
                score: arg_score.score
            })
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_uri = std::env::var("REDIS_URL")
        .unwrap_or(String::from("redis://default:redispw@localhost:6379"));

    println!("using redis url {}", redis_uri);

    let app_host = std::env::var("HOST")
        .unwrap_or(String::from("0.0.0.0"));

    println!("using app host {}", app_host);
    let app_port: u16 = std::env::var("PORT").unwrap_or("50051".to_string()).parse().unwrap();
    let client = redis::Client::open(redis_uri).unwrap();
    let ip: Ipv4Addr = app_host.parse().unwrap();
    let socket: SocketAddr = SocketAddr::new(IpAddr::from(ip), app_port);
    let s = Service::new(client);

    println!("server up & running at {} : {}", socket.ip(), socket.port());

    Server::builder()
        .add_service(LeaderboardServer::new(s))
        .serve(socket)
        .await?;

    Ok(())
}
