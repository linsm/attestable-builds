# Artifact Evaluation Walkthrough

This document provides a step-by-step guide to walk you through the artifact evaluation. 
The main objective is to reproduce the evaluation results, presented in our paper. 

The experiments provide time estimates in human-hours, or human-minutes (can be interrupted), and machine-hours, or machine-minutes (cannot be interrupted).

## Prepare the sample repository (5 human-minutes)

The first step is to prepare the Git repository including the sample projects used in our evaluation.
The following list provides a step-by-step overview of the necessary preparation steps:

1. Fork our [sample repository](https://github.com/linsm/ab-samples) to your own GitHub account. Make sure to copy all branches, i.e. remove the checkbox from `Copy the main branch only`
2. Log in to your GitHub account and navigate to the [Developer Settings](https://github.com/settings/apps) of your GitHub account 
3. Navigate to `Personal access tokens -> Fine-grained tokens` and click `Generate new token`
4. Give it a name and select `Only select repositories` and select the forked repository (ab-samples) 
5. Click on `Add permissions` and select the following permissions
	1. Actions with Read and write
	2. Administration with Read and write
	3. Commit statuses with Read-only
	4. Contents with Read-only
	5. Environments with Read-only
6. Generate the token and save it. This will be used later to configure the environment variable on the AWS instance.

## Prepare the AWS environment

The next step is to prepare the AWS environment, including the security group and the EC2 instance. 

### Create the security group (2 human-minutes)

1. Navigate to EC2 area / Networks & Security / Security Groups
2. Create a new security group with the following settings:
    - Give it a name (e.g., artifact-eval)
    - Add a description (e.g., Allow SSH and GitHub hooks)
    - Add a new Inbound rules (Custom TCP; Port range: 22; Source: "My IP" (note: if you have a dynamic IP then you have to change the security group everytime your ISP updates your IP)) 
    - Add another Inbound rule (Custom TCP; Port range: 8000; Source: Anywhere-IPv4)

### Create the Amazon EC2 instance (5 human-minutes)

This will be the machine where the experiments are executed. To set up the instance follow the steps below:

1. We recommend selecting the region `us-east-1`, available in the upper-right corner after logging in. 
2. Navigate to `EC2 area -> Instances` and click on `Launch instances`
3. Give it a name (e.g., artifact-eval)
4. Select `Amazon Linux 2023 kernel-6.1 AMI` 
5. Select Instance type: `m5a.8xlarge`
6. Create a key pair for SSH access (select RSA or ED25519)
7. Download your private ssh key and store it securely; adapt permissions (e.g., `chmod 600 <path-to-ssh-key>`)
8. Select the previously created security group in the Network settings area
9. Configure 64GiB gp3 Storage
10. Go To Advanced details and enable `Nitro Enclave`
11. Finally, launch the instance

### Prepare the Amazon EC2 instance (3 human-mintues + machine-minutes)

Now the EC2 machine can be prepared for running the experiments.

1. Connect to your instance via SSH (the public IPv4 address is shown in AWS Portal -> Instances) (<1 human-minute)

   ```
   ssh -i <path-to-ssh-key.pem> ec2-user@<public-ip-ec2-instance>
   ```

3. Install git: (<1 human-minute)

   ```
   sudo dnf install git -y
   ```

5. Clone our repository and change to it's directory: (<1 human-minute)

   ```
   git clone https://github.com/linsm/attestable-builds && cd attestable-builds
   ```

7. Run the preparation script to install necessary dependencies and configuring the system: (4 machine-minutes)

   ```
   ./scripts/artifact-eval-setup.sh <INSERT REPOSITORY> <INSERT TOKEN>
   ```

   The repository name refers to the fork created in the [Prepare the sample repository](#prepare-the-sample-repository-5-human-minutes).
   This section also contains the creation of the token.

9. Reboot the machine and reconnect once it is back online. 

## Build the components 

After rebooting the machine, it is possible to start building the relevant components used for setting up the build environment.

1. Switch again to the cloned GitHub repository and run the setup for the AWS instance: (<1 machine-minutes)

   ```
   cd attestable-builds && make setup-aws
   ```

2. Install GO: (<1 human-minute)

   ```
   wget https://go.dev/dl/go1.25.0.linux-amd64.tar.gz
   sudo rm -rf /usr/local/go && sudo tar -C /usr/local -xzf go1.25.0.linux-amd64.tar.gz
   export PATH=$PATH:/usr/local/go/bin
   ```

3. Build the third-party libraries: (2 machine-minutes)

   ```
   make build-third-party
   ```

4. Build the EIF file for the enclave: (9 machine-minutes)

   ```
   make build-enclave-eif
   ```

5. Build the EIF file for the enclave without the inner sandbox: (8 machine-minutes)

   ```
   make build-enclave-wet-eif
   ```

6. Cleanup, build the evaluation setup and prepare the runner: (2 machine-minutes)

    ``` 
    sudo docker system prune -a -f
    make build-eval
    ./scripts/prepare-action-runner-for-local.sh
    chmod o+rx ~
    ```

## Setup the webhook (1 human-minute)

The next step is to create a GitHub webhook on the forked sample repository. 

Navigate to `https://github.com/ORGANIZATION/REPOSITORY/settings/hooks/new` and replace the ORGANIZATION and the REPOSITORY accordingly.
The webhook can be created with the following configuration:

```
Payload URL: `http://<PUBLIC-IP-AWS-INSTANCE>:8000
Content type: `application/json`
SSL verification: Disable
Active: Ticked
```    

## Run the test suites and the evaluation

The repository contains two test suites to verify the configuration before running the final evaluation.
The first test suite contains test cases where parts of the infrastructure is simulated (e.g., using a fake GitHub runner or webhook).

To perform the initial verification tests. The test is successful if the following line is printed: 

```
INFO host_server::log_publishing_service: [simulated] Received entry :)
```
As soon as this line is printed, the test can be aborted with CTRL+C.

```
make test-local-direct (1 machine-minute)
CTRL+C if the line above is printed.
make test-nitro-direct (1 machine-minute)
CTRL+C if the line above is printed.
make test-nitro-sandbox (1 machine-minute)
CTRL+C if the line above is printed.
make test-nitro-sandbox-plus (1 machine-minute)
CTRL+C if the line above is printed.
```

Next, it is also possible to execute smoke tests of the final evaluation run:

```
make eval-smoketest (15 machine-minutes)
make eval-smoketest-big ( machine-minutes)
```

The evaluation of the sample projects is separated into two scenarios - `eval-full-one-round` and `eval-full-big-one-round`. The following list provides an overview of the scenarios including the associated projects:  

- `eval-full-one-round`: GProlog, Hello, IPXE, Neovim, Scheme48, Libsodium, TinyCC, Verifier Client, XZ
- `eval-full-big-one-round`: Clang, Linux Kernel and Linux Kernel-LLVM

To perform the first evaluation, run:

```
make eval-full-one-round
```

To perform the second evaluation (big), run:

```
make eval-full-big-one-round
```









