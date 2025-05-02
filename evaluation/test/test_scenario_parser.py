import unittest
from pathlib import Path

from src.scenario_parser import parse_scenario_csv, RunnerMode, BuildTarget, Scenario


class TestRunnerStartMode(unittest.TestCase):

    def test_mappings(self):
        self.assertEqual(RunnerMode.LOCAL_DIRECT.to_start_mode(), 'local')
        self.assertEqual(RunnerMode.LOCAL_SANDBOX.to_start_mode(), 'local')
        self.assertEqual(RunnerMode.ENCLAVE_DIRECT.to_start_mode(), 'nitro')
        self.assertEqual(RunnerMode.ENCLAVE_SANDBOX.to_start_mode(), 'nitro')

        self.assertEqual(RunnerMode.LOCAL_DIRECT.to_runner_mode(), 'direct')
        self.assertEqual(RunnerMode.LOCAL_SANDBOX.to_runner_mode(), 'sandbox')
        self.assertEqual(RunnerMode.ENCLAVE_DIRECT.to_runner_mode(), 'direct')
        self.assertEqual(RunnerMode.ENCLAVE_SANDBOX.to_runner_mode(), 'sandbox')


class TestScenarioParser(unittest.TestCase):
    def setUp(self):
        self.scenario_path = Path(__file__).parent.parent / "scenario_test" / "scenario.csv"

    def test_parse_scenario_csv(self):
        scenario = parse_scenario_csv(str(self.scenario_path))

        # Verify we have a Scenario object with the expected runs
        self.assertIsInstance(scenario, Scenario)
        self.assertEqual(len(scenario.runs), 4)

        # Test the first run
        run1 = scenario.runs[0]
        self.assertEqual(run1.name, "local_direct_01")
        self.assertEqual(run1.runner_start_mode, RunnerMode.LOCAL_DIRECT)
        self.assertTrue(run1.fake_attestation)
        self.assertTrue(run1.big_job)
        self.assertFalse(run1.use_real_runner)
        self.assertEqual(
            run1.target,
            BuildTarget(
                subproject_dir="project_c_simple",
                branch_ref=None
            )
        )

        # Test the second run
        run2 = scenario.runs[1]
        self.assertEqual(run2.name, "local_sandbox_01")
        self.assertEqual(run2.runner_start_mode, RunnerMode.LOCAL_SANDBOX)
        self.assertTrue(run2.fake_attestation)
        self.assertTrue(run2.big_job)
        self.assertFalse(run2.use_real_runner)
        self.assertEqual(
            run2.target,
            BuildTarget(
                subproject_dir="project_c_simple",
                branch_ref="branch"
            )
        )

        # Test the third run
        run3 = scenario.runs[2]
        self.assertEqual(run3.name, "enclave_direct_01")
        self.assertEqual(run3.runner_start_mode, RunnerMode.ENCLAVE_DIRECT)
        self.assertTrue(run3.fake_attestation)
        self.assertTrue(run3.big_job)
        self.assertFalse(run3.use_real_runner)
        self.assertEqual(
            run3.target,
            BuildTarget(
                subproject_dir="project_c_simple",
                branch_ref=None
            )
        )

        # Test the fourth run
        run4 = scenario.runs[3]
        self.assertEqual(run4.name, "enclave_sandbox_01")
        self.assertEqual(run4.runner_start_mode, RunnerMode.ENCLAVE_SANDBOX)
        self.assertTrue(run4.fake_attestation)
        self.assertTrue(run4.big_job)
        self.assertFalse(run4.use_real_runner)
        self.assertEqual(
            run4.target,
            BuildTarget(
                subproject_dir="project_c_simple",
                branch_ref="brunch"  # hihi
            )
        )


if __name__ == '__main__':
    unittest.main()
