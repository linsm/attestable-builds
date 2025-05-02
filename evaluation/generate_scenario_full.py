import csv

# General targets for the small machine
TARGETS = [
    "project_libsodium",
    "project_tinycc",
    "project_verifier_client",
    "project_xz_tar",
]

# Projects with different job count variants (also for the small machine)
SCALAR_TARGETS = [
    f"{target}_j{i}"
    for i in range(1, 8+1)
    for target in ["project_xz_tar", "project_verifier_client"]
]

# Everything we want to run on the "big" machine during an eval-full-big run
BIG_TARGETS = ("_big", [
    "project_clang",
    "project_kernel",
    "project_kernel_llvm",
])

# The new targets based around Debian packages
NEW_TARGETS = ("_new", [
    "project_gprolog",
    "project_hello",
    "project_ipxe",
    "project_menu",
    "project_scheme48",
    "project_neovim",
])

# Everything we want to run on the "small" machine during an eval-full run
SMALL_TARGETS = ("", TARGETS + SCALAR_TARGETS + NEW_TARGETS[1])

# We want to checkout each project individually in their branch to only download the required sources
USE_BRANCHES = True

# Whether to use the GitHub runner
USE_TRUE_RUNNER = True

RUNNER_CONFIGS = [
    "local_direct",
    "local_sandbox",
    "enclave_direct",
    "enclave_sandbox",
    "enclave_sandbox_plus",
]

ITERATIONS = 3

# Configurations that are in the TEE and therefore have access to NSM / real attestation
NSM_CONFIGS = ["enclave_direct", "enclave_sandbox", "enclave_sandbox_plus"]


def main():
    print("[ ] Generating scenarios")
    for suffix, targets in (SMALL_TARGETS, BIG_TARGETS, NEW_TARGETS):
        is_big = "_big" in suffix
        with open(f"scenario_full{suffix}/scenario.csv", mode='w') as scenario_file:
            scenario_writer = csv.writer(
                scenario_file,
                delimiter=',',
                quotechar='"',
                lineterminator='\n',
                quoting=csv.QUOTE_MINIMAL,
            )
            # name,runner_start_mode,fake_attestation,big_job,use_real_runner,target
            scenario_writer.writerow([
                "name",
                "runner_start_mode",
                "fake_attestation",
                "big_job",
                "use_real_runner",
                "target",
            ])
            for iteration in range(ITERATIONS):
                for target in targets:
                    for runner_config in RUNNER_CONFIGS:
                        fake_attestation = ("false" if runner_config in NSM_CONFIGS else "true")
                        scenario_writer.writerow([
                            f"{target}_{runner_config}_{iteration+1}",
                            runner_config,
                            fake_attestation,
                            ("true" if is_big else "false"),
                            str(USE_TRUE_RUNNER).lower(),
                            f"{target}@{target}" if USE_BRANCHES else target,
                        ])
    print("[+] Scenarios generated successfully")


if __name__ == "__main__":
    main()
