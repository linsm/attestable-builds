# attestedbuild-personality

## Setup Google Trillian

This section describes how to set up the backend of the transparency log. 
We use [Trillian by Google](https://github.com/google/trillian) to run the necessary services to host and manage the Merkle tree.

First, clone the trillian repository
```shell
git clone https://github.com/google/trillian.git
cd trillian
```

Afterwards, set a random password for the mysql instance:
```shell
export MYSQL_ROOT_PASSWORD="$(openssl rand -hex 16)"
```

To finally start the trillian services, run the following docker compose command:
```shell
docker-compose -f examples/deployment/docker-compose.yml up
```

You can verify if the instance is up by visiting the following URL:
```
localhost:8091/metrics
```

## Setup Personality

The personality is the application-specific part of the transparency log component. 
There are two ways you can run the personality. 

### Local 

One way to run the personality is to run it on your local machine directly. 
To do that you have to install rust, build the personality, and run it. 

Installation of rust:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build the personality:
 
```shell
cd personality/
cargo build
```

Configure the environment variable:

You will find a .env-template file to the root directory of your personality:

```shell
scp src/.env-template .env
```

Modify the .env file according to your settings. You can generate individual passwords here: 

```yaml
TRILLIAN_URL=<https://localhost:8090>
ADMIN_PASSWORD=<insert password here>
BUILDSERVER_PASSWORD=<insert password here>
TOKEN_SECRET=<insert token secret here>
```

### Docker

Another way to host the personality is to use docker and docker compose.
Follow the installation guideline for [Docker Engine](https://docs.docker.com/engine/install/). 

Now you can build the docker image, but first you have to adapt the .env configuration as described in the previous section. 

```shell
sudo docker build -t attestablebuilds-personality personality/
```

Make sure the docker container is using the same docker network as the trillian service is using. 

```shell
docker run -d --name attestablebuilds-personality --network deployment_default attestablebuilds-personality
```

## Endpoints

The personality service provides the following endpoints:
  
Endpoint: `/login/request-access-token`  
Method: `POST`  
Description: Requests an access token for authentication.  
Return: Authorization Token
Request Body:  
```json
{
  "name": "username",
  "password": "password"
}
```

Endpoint: `/log/list-trees`  
Method: `GET`  
Description: Lists vailable trees on the log server. 
Return: List of available trees.

Endpoint: `/admin/create-tree?description=<INSERT DESC>&name=<INSERT NAME>`  
Method: `POST`  
Description: Creates and initializes a new tree.   
Header: `Authorization: Bearer <Insert Token>` for admin 
Return: Log ID

Endpoint: `/logbuilder/add-logentry?log_id=<INSERT LOG ID>`  
Method: `POST`  
Description: Creates a new log entry.
Header: `Authorization: Bearer <Insert Token>` for buildserver
Request Body:  
```json
{
    "commit_hash": "commit hash",    
    "artifact_hash": "artifact hash",
    "artifact_name": "artifact name",
    "attestation_document": "attestation document"
}
```

Endpoint: `/log/inclusion-proof?log_id=<INSERT LOG ID>&tree_size=<INSERT TREE SIZE>`  
Method: `POST`  
Description: Requests an inclusion proof for a given log entry.
Request Body:  
```json
{
    "commit_hash": "commit hash",    
    "artifact_hash": "artifact hash",
    "artifact_name": "artifact name",
    "attestation_document": "attestation document"
}
```
## Publication

This repository contains source code referenced in a scientific paper. The respective paper will be submitted to the [32nd ACM Conference on Computer and Communications Security](https://www.sigsac.org/ccs/CCS2025/).

## LICENSE

While the respective paper is under review, please consider this repository as confidential and under the following license: 

All rights reserved.
